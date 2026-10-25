---
title: "How results are checked"
weight: 45
---

This page explains why a result from AL Runner can be trusted, how that is checked
against real Business Central, and where the known risks are.

## Built on the language, not on Business Central's internals

AL Runner reproduces what AL code can observe: values, errors, data, and the order
of triggers and events. That is the public contract of the AL language. Microsoft
cannot change it without a breaking change for every partner, so it changes slowly
and visibly, behind versions and obsolete markers.

A runner can instead run Microsoft's compiled code on top of Business Central's
internals: its C# runtime, its metadata and session classes. Microsoft can change
those in any hotfix or release, without notice. Such a runner needs one engine per
Business Central minor version. AL Runner is one binary for all supported versions.

Where observable behavior differs between versions (an error text, a column of a
system table), it is measured on every major and pinned per version. A change in a
new release then shows up as a failing check with a known cause. The one part that
still loads Microsoft's internals is the optional .NET helper for `DotNet` code. It
is kept small on purpose.

## What "correct" means

Correctness is defined by real Business Central, not by documentation. Behavior
that AL code can observe must match Business Central. Speed, diagnostics and tooling
are free to be better.

## How results are checked against real Business Central

- **The AL language test corpus.** The
  [corpus](https://github.com/aldevtools/BusinessCentral.AL.Language.Tests) is a
  collection of small AL tests, each of which pins one behavior. Its CI runs it on
  real Business Central 27.5, 28.5 and 29.0. The result of each run is the reference
  for that version. A corpus test counts as passing only when it passes in AL Runner
  **and** passed on real Business Central.
- **No step back.** A list of corpus tests that must keep passing is kept for each
  version. CI runs all three versions on every pull request and fails when one of
  them stops passing.
- **Code that must not compile.** The corpus also holds small AL files that the
  real AL compiler rejects. AL Runner must reject each one with the same diagnostic
  codes, on each version.
- **Linux checked on Windows.** The corpus CI runs Business Central in a Linux
  container, which Microsoft does not support. Results that look like a container
  difference are run on Microsoft's Windows container first. Where the two differ,
  AL Runner refuses that case.
- **Measure first.** Every behavior AL Runner implements is measured on real
  Business Central by a corpus test first. When nothing measures a behavior yet, AL
  Runner refuses the test and reports it as not run. It does not guess.

The one exception: a behavior that Microsoft's own tests need may be built before a
corpus test measures it. Each such behavior is registered as an unprobed assumption,
and a pass that relies on one is reported as a **flagged pass**. See [Reading
results](results.md#flagged-passes). `al-runner --list-assumptions` lists all of them.

The current numbers are in [What works and what is refused](what-works.md#status).

## Known risks

1. Some behavior was written before the measurement rule and has no corpus test
   yet. Corpus tests are being added for it.
2. A flagged pass depends on its assumption. When a corpus test measures the
   behavior and disagrees, AL Runner is changed to match Business Central.
3. The Linux container can differ from Business Central on Windows. Only results
   that someone suspects are run on Windows.
4. Microsoft's own tests are not compared with real Business Central test by test.
   Their pass rate is the result of AL Runner, not a measured match.
5. Permissions are not enforced. Every access is granted, so a test that expects a
   permission error fails here, and code that Business Central would stop for a
   missing permission runs here.
6. The "not trusted" rule covers failures only. A test that passes after a refused
   test stays a pass, even if the refused test would have changed its data on
   Business Central.

If you see a result that differs from real Business Central, please report it. See
[Reporting a problem](feedback.md).
