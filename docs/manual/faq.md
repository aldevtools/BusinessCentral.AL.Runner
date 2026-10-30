---
title: "Questions and answers"
weight: 132
---

## Does AL Runner replace a Business Central container?

No. It is a testing tool for fast feedback on your machine and in pull request
builds. Before you deploy to production, run your tests at least once in a way
Microsoft supports, for example in a Business Central container on Windows
(BcContainerHelper) or with AL-Go for GitHub.

## What do I need to install?

Only the `al-runner` binary. It needs no Docker, no SQL Server, no service tier and
no .NET. Tests that use the `DotNet` type need the .NET 8 SDK; see
[What works](what-works.md#dotnet). [Getting started](getting-started.md) has the
install steps.

## Do I have to change my tests?

No. AL Runner runs ordinary test codeunits (`Subtype = Test`) with the Microsoft
test libraries. See [Writing tests](writing-tests.md).

## Which Business Central versions are supported?

27.5, 28.5 and 29.0, with one binary. See [Supported versions](versions.md).

## Why are some of my tests "not run"?

AL Runner refuses a test when it cannot answer the way Business Central would. It
never guesses. Each refused test has a reason. Run with `--top-refusals`, or
`--explain TestName` for one test. See [Reading results](results.md).

## Why does the first run download so much?

Your app depends on Microsoft's System app, and by default the run restores the
CRONUS demo company (about 520 MB). Both are kept in the cache folder. If your tests
need no CRONUS data, add `--no-test-data`. See [Getting started](getting-started.md#the-first-run).

## Can I use it in a build pipeline?

Yes. It runs on an ordinary hosted runner. See [Continuous integration](ci.md).

## Are permissions checked?

No. Every access is granted. A test that lowers its own permissions is not run,
unless you pass `--ignore-permission-mock`. See [Writing tests](writing-tests.md#permissions).

## Can I use a dependency that ships without AL source?

No. AL Runner compiles every app from its AL source. Tests that reach an app without
source are not run, and the message names the app.

## Does it work in my editor?

Tools such as ALchemist use `al-runner serve` and `al-runner debug`. See
[Editor and tool integration](editor-tools.md) and [Debugging](debugging.md).

## Where do I report a wrong result?

Open an issue. [Reporting a problem](feedback.md) says what to include.
