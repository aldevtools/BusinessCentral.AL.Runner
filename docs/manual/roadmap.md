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

- **App translations from packages.** The `Translations/*.xlf` files of an app folder
  are applied after a test switches `GlobalLanguage` away from English (1033): labels,
  enum value captions and field captions. The translation files inside a package (`.app` or source
  archive), and the reads not built yet, are still refused.
- **`Image.Resize` of the System Application**
  ([issue 5590](https://github.com/aldevtools/BusinessCentral.AL.Runner/issues/5590)).
  Tests that resize an image are refused.

- **Test handlers (BC 29)**
  (ADR-059). Codeunits with `TestHandlers` are not run. Planned: a "skipped" result
  for tests that a handler skips.
- **Permission checks against a license** (ADR-060 and ADR-062, planned). `License Permission`
  already answers from the license file (see [BC license](license.md)). Table data writes in
  a session that a test lowered are checked. Planned: reads, `Execute` permission, the
  permission sets of the runner, and the license as a limit on what a permission set grants.

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

These features of version 2 do not exist in this version yet:

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
- Signed macOS and Linux binaries. The Windows binary is signed. Every release carries
  `SHA256SUMS` and build provenance attestations.
- Builds for platforms other than Linux x64, Linux arm64, Windows x64 and macOS arm64.

## Decided against

- **Watch mode** (`--watch`) and its live terminal dashboard. `serve` keeps the
  compile warm, and `--affected` runs only what a change can affect. `--watch` is
  refused with a reason. See [Editor and tool integration](editor-tools.md).
