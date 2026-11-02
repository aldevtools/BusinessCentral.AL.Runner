# ADR-064: A refused test untrusts only the later tests that read what it could have written

- Status: Accepted (by the project owner, 2026-11-02; bound only)
- Date: 2026-11-02

## Context

Tests of one codeunit share state: the codeunit's globals, single-instance codeunits, the database (under
`TestIsolation = Codeunit`) and session state (`Random`, `WorkDate`, language, number sequences, isolated storage,
bound subscribers). When an earlier test is refused (or ends without running to its end), the state it would have left
is not BC's. Later tests of the codeunit may then behave differently from BC.

What the runner does today (ADR-043, "untrusted-failure-2" and "untrusted-failure-3"):

- It keeps a list of unfinished tests per codeunit (`al-testrun/src/runtime/run.rs:571-577`: refused before it started,
  refused while it ran, failed).
- It checks only a later **failure** (and a later time-out), at `run.rs:982-1056`. If an unfinished test's static
  reach can change what the failing test reads, the failure becomes "not run" with the reason "this failure is not
  trusted". The check is `ReachFacts::changes_read_by` (`al-testrun/src/runtime/written.rs:149-211`). It compares
  writes of the unfinished test with reads of the later test: session-state bits, globals of codeunits, and tables
  (not under Function isolation).
- A later **pass** is never checked. It stays a clean pass.
- If any part of the unfinished test's reach is unknown (a refused routine, a run by an id known only at run time),
  the check answers "may change anything" without looking at the later test (`written.rs:158-162`).

So the rule the owner wants ("a refusal untrusts only later tests whose reads meet the refused test's writes") is
already built for failures. This ADR records it as the rule, names what is still open, and proposes two changes.

### Measured (read-only, no new run)

Source: the JUnit files of the last full BC 29 Microsoft run in `~/.cache/al-runtime/msruns/ms29-roc/` (35 bucket
files, 2026-10-31, 39 457 test cases), counted with a script over `<skipped message=...>`. The run is two days old; the
rule code has not changed since. Not re-run, because the machine is busy with measurement runs.

- 1 581 tests are "not run" because of "this failure is not trusted" (4 % of the tests).
- In 1 578 of them the reason is "part of its reach is not known ... so it may change anything". Only three tests
  are untrusted because of a concrete overlap. The overlap check hardly matters in the Microsoft buckets. The unknown
  parts decide.
- The largest unknown parts: a subscriber refused for a `[RunOnClient]` variable (Office Host Provider
  `OnGetHostType` and `OnIsAvailable`, about 816 tests); `Init. Server Printer Table.GetLocalDefaultPrinter`, refused for
  the same reason (330); a run of a page that is not in the program (156); `Page.RunModal` and `Xmlport.Export` with an
  id known only at run time (about 100).
- Both big causes are small routines whose bodies write no table (`OfficeHostProvider.Codeunit.al:77-86` sets a `var`
  parameter; `InitServerPrinterTable.Codeunit.al:81-96` only reads a client printer name), but the runner treats a
  refused routine as "may change anything".
- 10 824 passing tests come after a not-run test (not one of the untrusted failures) in the same codeunit, in 518
  codeunits. Today these are clean passes. How many would meet a write of the refused test is not measured.

## Decision

### 1. The rule

A test that did not run to its end (refused before it started, refused while it ran, failed) affects a later test of
the same codeunit only through state the later test reads and the earlier test could have written. Everything else is
independent of it and is judged as if the earlier test did not exist.

This is today's rule for failures, kept as is. Two things change.

### 2. What counts as "reads meet writes" (unchanged, listed so it can be reviewed)

Both sets are computed statically, per test, from the routines reachable from the test (`written.rs`,
`ReachFacts`). The reach follows calls, triggers of the tables the test's records use, routines run by a constant id,
and the subscribers of every event any of them raise.

| State | Written when the routine | Read when the routine |
|---|---|---|
| Table | Inserts, modifies, deletes, renames a record of a known table (a source table of a page counts) | has a record of the table in a slot, or a query reads it |
| Codeunit global (incl. single-instance) | assigns it, passes it as `var`, calls a method on it, clears it | names it |
| `Random` | draws or reseeds | draws (judged by what the later test did when it ran) |
| WorkDate, language, session settings, number sequences, isolated storage, bound subscribers | changes it | reads it; WorkDate, language and bound subscribers count as read by every test |
| Files, HTTP, other (.NET object, background task, unclassified builtin) | does the operation | does the operation (other: read by every test) |

Two exclusions exist: tables do not count under Function isolation (the database is restored), and a global that
holds the only value any earlier test can have left in it ("settled") does not count.

"Meets" means the two sets share an element, or the written set is "any table" and the later test touches a table.
Events bound count on both sides.

### 3. Unknown sets: bound them, otherwise keep today's behavior

An unknown part of a refused test's reach still counts as meeting everything. This is today's behavior and stays the
default, because the alternative is a false result (section 5).

Change (proposed): an unknown part gets a **bound** when the refused routine's own source shows it cannot write.
A routine counts as write-free when its syntax tree has no call except to builtins classified as pure or read-only, no
record write operation on a non-temporary record, no assignment to a global of a codeunit or table, no event
publisher call, no `Codeunit.Run` and no run of another object. Such a routine contributes only the reads it names.
It still stays refused; this changes only what it is assumed to have left behind. The check works on the syntax tree
because the lowered body may not exist for a refused routine.

The two biggest causes in the measurement (the Office Host Provider subscribers and `GetLocalDefaultPrinter`) pass
this check as the source reads today. The bound is therefore an upper limit on the tests that can move: at most 1 146
of the 1 581 (72 %). The real number is lower, because each of those tests can still meet a write elsewhere in the
earlier test's reach. It is not measured; the first implementation step prints it before anything is enabled.

A run of a page, xmlport, report or codeunit that is not in the program, or whose id is known only at run time, stays
unknown. Its body could write anything.

### 4. Reporting

- A test that is untrusted keeps the reason text as today ("an earlier test of this codeunit was refused ... : <what>,
  so the state it would have left is not BC's; this failure is not trusted").
- A test moved out of "not run" by the bound is reported as an ordinary failure, with the new line
  "after a refused test: <name> (judged independent: it cannot write)".

### 5. Risk: a false pass

A false pass is a later test that passes here only because the refused test's writes are missing, while on BC it
fails. How the rule prevents it:

- Moving a test out of "not run" yields a **failure** or a **pass**, never a pass created from nothing: the test
  still ran. The tests the bound moves today are failures here (1 581 are untrusted failures). They stay failures, they
  are only no longer hidden. A failure caused by missing state is a false failure (the test would pass on BC). The
  report line in section 4 tells the reader to check it.
- A false pass needs a test that passes without the refused test's writes and fails with them. Such a test reads the
  state the refused test writes. Passes are not checked (see Deferred).
- The bound in section 3 is sound only if the write-free check is conservative. It is: any call to a routine that is
  not classified makes the routine "unknown". A test in the program that makes the check wrongly say "write-free" is
  a bug in the check and is covered by the tests below.
- Unknown stays "meets". No guess is made about a page run by a variable id.

### 6. Cost

The failure side adds no walk: it already walks only unfinished tests. The bound adds one syntax-tree scan per refused routine, cached per routine.

### Deferred

Flagged passes after a refused test: deferred until their count and the reach-walk cost are measured (owner, 2026-11-02).

## Consequences

- Fewer failures are hidden for reasons that do not matter (a refused host-only subscriber). Reports say "fails"
  where the runner can show the cause is not shared state.
- The bound moves an upper limit of 1 146 Microsoft tests from "not run" to a verdict. Each is a gap finder,
  not a quality claim (whole-language goal).
- Risk accepted: the write-free check depends on the syntax tree walk being complete for new syntax. It is a
  closed list; an unknown node kind makes the routine unknown.

## Conformance

- Existing tests stay: `failure_after_refused_test`, `failure_after_unfinished_test`, `timeout_after_refused_test`
  (`al-testrun/tests/suite/`), and the unit tests in `al-testrun/src/runtime/tests.rs` near line 2782.
- New, red first: a refused subscriber that only sets a `var` parameter does not untrust a later failing test; a
  refused routine that inserts a record does; a refused routine that calls an unclassified routine does.
- Differential: a full BC 29 Microsoft run before and after must differ only in tests that were "this failure is
  not trusted" (now a verdict) only. Zero lost baseline cases.
- Implementation step 1 (no behavior change) prints how many tests the bound would move; the owner sees the number before step 2 enables it.
