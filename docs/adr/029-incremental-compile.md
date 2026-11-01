# ADR-029: Incremental compile of the warm workspace (a change of code compiles only its objects)

- Status: Accepted (owner direction for live testing as you type, `docs/ideas.md`, and ADR-026 option 3; accepted by the
  coordinator on 2026-10-15)
- Date: 2026-10-15
- Related: ADR-026 (analysis; advised against, before the warm server existed), ADR-028 (warm server), ADR-022 (compile
  cache), ADR-013 (demand-driven compile)

## Context

`alrunner serve` keeps the compile of the last request (ADR-028). A source change used to compile the whole workspace
again: 2.7 s for Tests-ERM (290 files), most of it `check` of the workspace (0.8 s), the load (0.8 s, parse and
reference resolution), lowering (0.3 s) and the read of the dependency blocks (0.4 s). A developer who types a line
into one codeunit pays all of it. ADR-026 judged a general per-object incremental compile too costly (a read log, a
positional IR, a stored result). A warm process changes two things: nothing has to be stored, and the whole earlier
compile is at hand.

## Decision

1. **Scope: a change in the code of routines of codeunits.** The new text of a workspace file is taken in place when
   - the file is a workspace file and the earlier compile used the dependency cache (the layered one of ADR-022),
   - the new text parses without errors,
   - the declarations are the same: every token outside the `var` sections and `begin ... end` blocks of procedures and
     triggers (`incremental::skeleton`). So objects, fields, procedure signatures, attributes, properties and global
     variables are the same; adding or removing a local variable is a change of code,
   - every object of the file is a codeunit that implements no interface, a table or a table extension
     (`incremental::eligible`),
   - the tests do not look free of data (otherwise a complete compile would weigh the install baseline first, see
     `text_looks_pure`, and the update says no).

   Anything else compiles all of it again, as before. Not covered yet: pages, reports and other kinds (the lowering of
   a page or report reads the code of its extensions), files added or removed, a changed selection, and the compile
   without the dependency cache (the demand-driven path for tests free of data, which costs 0.5 s anyway).
2. **What is renewed.** `Workspace::replace_file` swaps the tree, the offsets of the file's objects and the file's
   reference diagnostics. `check_with` runs on the objects of the file only (`Roots::only_objects`; the left-out test
   codeunits of the selection keep their status except where the text changed). The tables the lowering reads are built
   again for all objects (`build_shared`, 0.04 s on ERM), and only the objects of the file are lowered
   (`relower_objects`). Their IR replaces the old one in the program in place, the table of source positions is built
   again in object order, the diagnostics of other files stay. The install baseline decision is made again.
3. **Why this equals a complete compile.** The lowering of an object reads its own syntax and the declarations of the
   others, never their code; every reference from one object to another in the IR is an object or routine number, and
   a change of code leaves the numbers as they are. The offsets of routines in a file do move, but they are looked up
   when the file is lowered. The positions are renumbered exactly as the assembly of a complete compile numbers them.
4. **Proof.** `Compiled::same_lowering_as` compares the program, why routines are refused, the calls, the diagnostics
   and the baseline decision. The test `crates/al-testrun/tests/suite/incremental.rs` runs edits (an expression, a statement
   more, a literal, a local variable more, a comment, a statement in a procedure with a `var` parameter, a change of a
   left-out test codeunit that makes it stay) and compares after each with a complete compile of the same texts; a
   declaration change and a text that does not parse compile everything again. `ALRUNNER_CHECK_INCREMENTAL=1` makes
   `alrunner serve` do that comparison after every update, on real input.
   `ALRUNNER_NO_INCREMENTAL=1` turns the update off.
5. **The wire.** The summary of a request answered by an update has `"incremental": true` (and `cached: false`).

## Measurements (release, 4 threads, the machine at a load average of 22 to 35, so read the pairs, not the seconds)

The same server session, one source file changed by a comment at its end; `ALRUNNER_NO_INCREMENTAL=1` for the first
column of each pair. "Request" is the whole `runTests` request, "first test" the time to the first `test` line.

| Input | Request, all compiled again | Request, update | First test line | RSS after |
|---|---:|---:|---:|---:|
| Tests-ERM (290 files), codeunit 134000 (32 tests) | 5.48 s | 2.21 s | 3.62 s / 0.39 s | 1952 MB / 1406 MB |
| corpus app (1 604 files), 2 tests, cache path | 1.34 s | 0.37 s | 1.30 s / 0.36 s | 1248 MB / 951 MB |

The update itself (`incremental: ... all`) took 0.056 s on ERM and 0.093 s on the corpus app, against a compile of 2.5 s and
1.0 s. What is left in a changed request is not the compile: reading the workspace files and finding the dependencies
(0.1 to 0.2 s), the install baseline (0.1 s), the engine, and the tests. A request that changes nothing takes the same time as
one that changes a line. A complete compile also holds two programs at once (about 550 MB more on ERM).

First request on a fresh server (the cache is on disk and hit): the compile cache is used as in a one-shot run. An earlier
reading of ADR-028 (7.6 s against 1.3 s) was a cache miss: the cache key holds the build of the executable, and the
server had just been built. A second server process on the same input: corpus app 1.36 s, Tests-ERM 4.25 s, the figures
of a one-shot run.

Differential on real input (`ALRUNNER_CHECK_INCREMENTAL=1`: after every update, a complete compile of the same texts must
give the same lowering): Tests-ERM, 140 updates (70 changes of codeunits and tables, in 80 % codeunits, and the 70 changes
back: a blank line, a comment, a local variable, a statement more, an error text), and the corpus app (1 604 files,
`AL_FORCE_BASELINE=1` to take the cache path), 220 updates of codeunits, tables and table extensions: 0 differences in
all. (Changes in files of other kinds compiled all of it again, as designed.) `tools/demand_vs_eager.sh`,
`tools/cache_vs_uncached.sh` and `tools/baseline_rule_check.sh`: 0 differences.

## Limits and what comes next

- Pages, reports, xmlports and queries are the next kinds. A page or report is renewed together with its extensions
  (the lowering of the base reads the extensions' controls and triggers); the same differential shows whether that holds.
- The workspace-wide pass for the AL compiler's codes runs in full each time (up to 0.09 s on ERM at high load); it
  could run on the changed file.
- The load of the request (reading the workspace files, finding the dependencies, 0.1 to 0.2 s) and the install baseline
  (0.1 s) run on every request; both can be kept when their inputs are the same.
- The compile without the dependency cache (tests free of data, `text_looks_pure`) compiles all of it again; it costs
  0.5 s on the corpus app.
- Memory: the update keeps no second copy of anything. It needs the program to be held only by the compile (the engine of
  the last request is gone); otherwise the program is copied once, which is slow and correct.

## Consequences

- A typed line in a codeunit costs the check and lowering of that codeunit, not of the workspace.
- A defect in the update would show as a result that differs from a complete compile. The comparison runs in tests and
  can run in every request; the update can be switched off.

## Amendment (ADR-033 step 5): declaration changes

A change of a declaration of a codeunit that implements nothing is taken in too. `Workspace::replace_file_declared`
takes the text when the objects of the file are the same ones (kind, id, name, what they extend and implement).
`incremental::readers` finds the objects of the workspace's own apps whose file mentions the name or the id of a
changed object (a reference is made by one of the two); they are checked and lowered again with the changed ones
(`relower_objects`), and `relink` resolves them against the program by key, so the new routine numbers of the changed
codeunit reach its callers. More readers than a fifth of the objects (and 20) compiles everything again, as do all
other kinds (a table's fields, a page's controls and an interface's implementers are read without a name).

Measured on Tests-ERM (`alrunner serve`, a procedure added to a test codeunit, one test requested; the machine busy):
request 3.95 s before (everything compiled again), 0.77 s after (the reuse of an unchanged request: 0.7 s). With
`ALRUNNER_CHECK_INCREMENTAL=1` (a complete compile after every update must give the same lowering): 12 codeunits
changed and changed back, 24 updates, 0 differences. The slowest (8.3 s, `TestPostingPreview`) has many readers.
