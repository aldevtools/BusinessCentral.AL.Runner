---
title: "Roadmap"
weight: 115
---

This page lists what AL Runner does not do yet, what waits until users ask for it,
and what was decided against. For what works today, see
[What works and what is refused](what-works.md).

## How to move an item up

Each item links to its GitHub issue where one exists. If you need an item, comment on
the issue and say what you need it for. Demand from users decides the order. If no
issue exists, open one (see [Reporting a problem](feedback.md)).

## Deferred until users ask

These are understood, but they wait for more user demand. Tests that need them are
reported as not run, with the reason.

- **App translations under another language**
  ([issue 5772](https://github.com/aldevtools/BusinessCentral.AL.Runner/issues/5772)).
  AL Runner does not apply an app's own `Translations/*.xlf` files. After a test
  switches `GlobalLanguage` away from English (1033), a read whose text a translation
  could change, such as `Format` of an enum value or a label, is refused.
- **`Image.Resize` of the System Application**
  ([issue 5590](https://github.com/aldevtools/BusinessCentral.AL.Runner/issues/5590)).
  Tests that resize an image are refused.
- **Business Central's development-endpoint test mode**
  ([issue 3697](https://github.com/aldevtools/BusinessCentral.AL.Runner/issues/3697)).
  This is the mode that suppresses unhandled UI. AL Runner does not offer it.

- **Test handlers and data-driven tests (BC 29)**
  (ADR-059). Codeunits with `TestHandlers` and `[TestDataSource]` procedures are
  not run. Planned: a "skipped" result for tests that a handler skips, and one
  result per data-driven case. The name of a case's result row waits for a
  measurement on a Windows Business Central container.
- **License-backed permissions** (ADR-060, planned). Today `License Permission`
  answers from a few pinned rows and one assumption for the rest. Planned: read the
  rows from a Business Central license file, by default the one in the artifact, and
  a `--license FILE` flag for a developer, partner or customer license. The flag does
  not exist yet. Permissions stay unenforced; enforcement is a later step.

## Waiting on a change in the Linux test setup

AL Runner checks each behavior against real Business Central before it implements it.
Some test isolation cases cannot be measured yet on the Linux Business Central setup
that runs those checks. Until they can, their tests are not run:

- The `RequiredTestIsolation` cases listed under isolation in
  [Writing tests](writing-tests.md).

## Experimental and off by default

- **Building Microsoft's demo company.** The builds of the DemoTool and Contoso
  companies are on hold and will be redesigned. They run only with
  `--experimental-company-builds`. Without it, `--company-data demotool`,
  `contoso-setup` and `contoso-all` are refused, and the Legacy, IntegrationTest and
  Uncategorized lanes of `--recipe microsoft` are reported as not run, with the
  reason. See [Test data](test-data.md).

## Not available yet

These features of version 2 do not exist in version 3 yet:

- Finding the app by climbing up from any folder below an `app.json`. Today a folder
  without an `app.json` is an error.
- Running a folder that holds several apps as one run per app. Today each app must be
  named. Only `--affected-from` splits a folder of test apps.
- Packaging a folder as an `.app` without compiling (`--emit-app`).
- Running a dependency that ships only compiled code, without AL source. Tests that
  reach such an app are not run, and the message names the app.
- `--no-strict-exit` (always exit 0). `--allow-not-run` turns only the exit codes 7 and
  8 into 0.
- A note under a failure that names a test-data table that is empty, or the real
  error behind "The TestPage is not open."
- Signed binaries. Releases carry `SHA256SUMS` and build provenance attestations.
- Builds for platforms other than Linux x64, Windows x64 and macOS arm64.

## Decided against

- **Watch mode** (`--watch`) and its live terminal dashboard. `serve` keeps the
  compile warm, and `--affected` runs only what a change can affect. `--watch` is
  refused with a reason. See [Editor and tool integration](editor-tools.md).
