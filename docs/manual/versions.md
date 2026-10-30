---
title: "Supported versions"
weight: 25
---

## Business Central versions

AL Runner is checked against Business Central 27.5, 28.5 and 29.0. One binary
serves all three. The [AL language test corpus](https://github.com/aldevtools/BusinessCentral.AL.Language.Tests)
runs on real Business Central of each of these versions, and AL Runner is measured
against each result. Other majors are not checked. Use one of these three.

## How the build of a run is chosen

`--bc-version V` names the version. It takes a major (`28`), a major and minor
(`28.5`), or an exact build (four numbers). Without it, AL Runner takes the first
rule that applies:

1. The build of the tests, when they are Microsoft's own test apps.
2. The build of the Microsoft packages in your package folders (`.alpackages`,
   `--dep`, `--package-cache`), when it is of the target major and not below the
   `application` and `platform` versions in `app.json`.
3. The newest published build of the target major, at or above those versions.

The target major is the major of `application` in `app.json`. Without it, the target
is the newest major AL Runner is checked against (29). `--verbose` prints which rule
chose the build.

To run the same tests on several versions, give a list:
`--bc-version 27.5,28.5,29.0`. See "Several versions and countries" in
[Writing tests](writing-tests.md#several-versions-and-countries).

## The `runtime` of your app

Each app sees the AL methods of the `runtime` that its own `app.json` declares. A
call of a method that is newer than that runtime gets the AL compiler's error
AL0666, and the run stops with exit code 3, as the compiler would. The same holds
for properties, data types, method attributes and object types. [Writing
tests](writing-tests.md) has the details.

## Platforms

`al-runner` is built for:

- Linux x64
- Linux arm64
- Windows x64
- macOS arm64

There are no builds for other platforms. The core needs no .NET. Tests that use the
`DotNet` type need the .NET 8 SDK; see [What works and what is refused](what-works.md).

## Release channels

- **Numbered alpha** (`3.0.0-alpha.N`): released by hand. Use it when you need a
  version you can name in a bug report.
- **`edge`**: rebuilt every night from the newest commit of `main` that passed CI.

AL Runner is an alpha. Results can change between builds. [Getting
started](getting-started.md) shows how to install each channel.
