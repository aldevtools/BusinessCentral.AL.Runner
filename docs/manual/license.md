---
title: "BC license"
weight: 62
---

A Business Central license file (`.bclicense`) says which object numbers are
licensed, per object type, and with which permissions. AL code sees it through
the platform table **License Permission** (2000000043). AL Runner reads a license
file at the start of each run and answers License Permission from it.

## Which license a run uses

1. `--license FILE`: the file you name. It can be a developer license, a partner
   license or a specific customer's license. It replaces the default for the
   whole run.
2. Otherwise, the license that ships with the BC artifacts of the run's build
   (the CRONUS demo license). AL Runner reads it from the artifact it already uses
   for Microsoft's apps: an extracted artifact in `~/.bcartifacts.cache/sandbox`,
   or the platform artifact on Microsoft's feed.
3. Otherwise, no license. This happens when the run uses no Microsoft app, or
   when the feed cannot be reached and no extracted artifact is on disk.

In `al-runner serve`, the `license` field of a `runTests` request is the
request's `--license FILE`.

A file that cannot be read as a license stops the run before any test starts,
with the reason. AL Runner never falls back to another license in that case.

## What AL Runner does with the file

- It reads the file into memory for this run. It never copies the file, never
  writes it into a cache and never uploads it. The compile cache and the stored
  install baseline keep only the file's SHA-256 hash.
- It does not check the file's signature.
- The run report names the license: its source, the licensee and expiry date
  from the file's header, and the SHA-256 hash. The summary prints one line
  (for a run on Microsoft's apps or with `--license`); the JUnit file carries
  `license.*` properties on every test suite; `--output-json` and the `serve`
  summary carry a `license` object (`source`, `path`, `licensee`, `expires`,
  `sha256`, `signatureChecked`, or `reason` when there is none).
- The expiry date is read under the English label `Expires` and the Danish
  label `Udløber`, and printed as the file writes it. If the header has a date
  under a label the runner does not know, the summary says `expiry not read
  (label not recognized)` and the reports add `expiryNotRead`; the runner does
  not guess which date is the expiry.

## What License Permission answers

With a license:

- A Get of any object number from 0 to 1999999999 answers from the license: the
  permissions of the range that holds the number, every permission blank for a
  number outside every range. `Table` rows come from the license's table
  description list.
- Range reads (`SetRange("Object Number", 400, 420)`) and `Count` within one
  object type over those numbers, and the first rows of the whole table.
- System objects (2000000000 and up) answer only where Business Central's own
  rows for them are known; other system numbers are not run.

When no license file is loaded, only the rows that are known from real Business Central are
answered; any other read is not run, with the reason.

Not run, with the reason: a negative object number, a filter on a permission
column, a read across several object types, the Limited Usage column of a
license other than the CRONUS demo license, and a range of the license that grants only
some of Read, Insert, Modify and Delete. A license that grants permissions
indirectly stops the run: its format is not read yet.

## What a license does not change yet

A loaded license changes only what License Permission answers. It does not limit
what the permission checks allow. Those checks cover table data writes in a session
that a test lowered; see [Writing tests](writing-tests.md#permissions). Checking your
permission sets and the `Permissions` property of your objects against a
customer's license is planned.
