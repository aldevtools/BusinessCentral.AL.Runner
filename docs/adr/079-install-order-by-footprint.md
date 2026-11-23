# ADR-079: Install order of partner apps proven by footprints, not by trying every order

- Status: Accepted (owner, 2026-11-23)
- Date: 2026-11-23
- Amends: ADR-046 amendment 7, decision 3 (at most three interacting partner apps, all their orders tried; more refuses).
  Decisions 1, 2 and 4 of amendment 7 and the comparison of amendment 8 item 2 stay. Amendment 8 item 1 (the watched
  run) stays until migration step 4 measures whether this rule covers it.
- Source: al-runner issue 8056 (reduced repro of six invented apps), the 23-app partner suite in the latest comments on
  issues 6924 and 8239.

## Context

BC keeps no install order for two apps with no dependency path between them (measured on 27.5, 28.5 and 29.0; owner,
2026-10-30), and there is no flag to state one (owner, 2026-11-02). So the runner may run the tests only when every
install order leaves the same database, and refuses otherwise. ADR-046 amendment 7 decides this by installing the
partner apps that met each other's code in every order and comparing the databases. It stops at three apps (3! = 6
orders) and refuses beyond.

Two outside suites now stop at that limit:

- **Issue 8056 (six invented apps).** Apps A and A2 each call `InitializeMappings` of a SingleInstance codeunit in a
  shared base app P, which raises `OnRegisterPrefix`. Apps B and B2 subscribe and add an entry to the `var`
  Dictionary. Four apps meet, so the run refuses, although the Dictionary lives in the install session and nothing
  reads it after the raise. With three of the four apps it passes.
- **A partner suite of 23 apps (issues 6924 and 8239).** One telemetry app has an `OnAfterLogin` subscriber that every
  partner install reaches through Company Triggers. All 23 apps end up in one group. About 8,700 of 11,611 tests do
  not run. The same tests pass on BC.

23! orders cannot be tried. Pairwise comparison is not enough either (amendment 7 decision 3: an effect may need three
apps in a certain order). The decision must stay "same result in every order, else refuse", without listing the
orders.

### What the code does today (main 46febc1f0)

- `al-testrun/src/install_orders.rs` `install_partner_apps`: installs the partner group once with the order of
  unordered pairs not known. A refusal site that meets another partner app's code (`dispatch.rs` event targets, `ctx.rs`
  `GetModuleInfo`, `records.rs` table extension triggers) notes the pair (`Session::note_order_unknown_hit`). The apps
  met grow into one set (`'grow` loop). Up to `MAX_INTERACTING` (3) apps: every order, rewound from a capture, compared
  by `Baseline::install_order_differences`. Beyond: one more install with the order not known, and what it reaches
  stays refused.
- `al-vm/src/order_trial.rs` (amendment 8 item 1): before it counts as a meet, a subscriber runs once under watch and is
  kept when it provably changes nothing lasting. The 8056 subscriber fails it ("it changes a var argument": the
  Dictionary is not a plain value). What made the telemetry subscriber fail is not known yet (open question 1).

### Facts this rests on (pinned)

1. **One install session per app, SingleInstance state fresh in it.** Each app's install runs in its own session; a
   SingleInstance codeunit starts from its initial state in each company and in the per-database part; the fixture
   app's SingleInstance counter reads 0 inside the main app's install (corpus `tests/al-language/record/BfInstallTests.al:125-140`,
   ADR-046 amendment 1 facts 7 and 8, A4). So SingleInstance globals, temporary tables, WorkDate, the language and
   other session state never carry from one app's install to another's. Only database state and the other lasting
   stores listed below do.
2. **One app's install ends before the next one begins** (ADR-046 A3, fact 4, `BfTests` P5).
3. **A subscriber or table extension of an app runs during another app's install exactly when its app was installed
   first** (corpus `tests/al-language/install-upgrade/install-triggers/Y79InstallUnrelated.al:6-12`,
   `install-upgrade/install-order/InstallOrderSubscriberTests.al:66-79`).

The rule below assumes nothing else about BC. It is a proof that the orders cannot differ, built from these facts and
from what the runner observes. It needs no new probe.

## Decision

### 1. Footprint of an install session

While a partner app's install code runs, the runner records the **footprint** of its install session: every lasting
resource the session reads or writes, and every app whose presence it can observe. Code of other apps that runs in the
session (subscribers, table extension triggers, called codeunits) counts for the session. A resource is one of:

| Resource | Key | Read by | Written by |
|---|---|---|---|
| Table row (not temporary) | company (empty for `DataPerCompany = false`), table, primary key | `Get` (found or not), the row a `Modify`/`Delete`/`Rename` reads | `Insert`, `Modify`, `Delete`, `Rename`, through Record, RecordRef or Variant |
| Table extent | company, table | every filtered read: `Find*`, `Next`, `Count`, `IsEmpty`, `CalcFields`/`CalcSums` source tables, `ModifyAll`/`DeleteAll`, `DataTransfer` | `Insert`, `Delete`, `Rename`, `ModifyAll`, `DeleteAll`, `DataTransfer` |
| AutoIncrement counter | company, table | an insert into the table | an insert into the table |
| Number sequence | name, company | `NumberSequence.Next`/`Current`/`Exists` | `Insert`/`Delete`/`Restart`/`Next` |
| Isolated storage | owning app, data scope, company or user, key | `Get`, `Contains` | `Set`, `SetEncrypted`, `Delete` |
| Row version | one for the database | a read of `SystemRowVersion`, `LastUsedRowVersion`, `MinimumActiveRowVersion` | every write of a row |
| App presence | the app | `NavApp.GetModuleInfo`, NAV App Installed App and the other app tables | - |

Two writes to the table extent do not conflict with each other (two inserts of different keys commute); a write to
the extent conflicts with a read of the extent. SystemId, the GUID counter, SystemCreatedAt/By and SystemModifiedAt/By
are left out as in amendment 7, and clock values as in amendment 8 item 2.

**Anything the runner cannot attribute counts as touching everything:** a page, report or XMLport run, .NET, files,
HTTP, `StartSession`, `TaskScheduler`, a Job Queue run, the Company table or a company copy, a resource the footprint
recorder does not classify. A session with such a footprint conflicts with every other session (and then refuses as
today when the group is larger than three).

**A meet** is what amendment 6 item 2 refuses today: while X installs, code of an unordered app Y would run (an event
subscriber, a table extension trigger or InitValue) or X observes Y's presence. A meet adds Y to X's **inputs**. It is
no longer a refusal by itself.

### 2. The influence graph

For two partner apps X and Y with no dependency path between them:

- **X reads from Y** when X's session meets Y, or reads a resource Y's session writes.
- **X and Y collide** when both sessions write the same row, AutoIncrement counter, number sequence or isolated storage
  key.

The **units** of the group are the strongly connected components of the graph "reads from" plus "collide" (both
directions). A unit of one app is plain. A unit of two or three apps is a **tangle**: its orders are all tried and
compared as amendment 7 does today, the tangle counts as one app in what follows. A unit of more than three apps
refuses as today (the baseline note names the apps and the resource that ties them).

### 3. Subset runs

Between units, "reads from" is now acyclic. For each unit X, its inputs I(X) are the units it reads from. X's install
runs once per subset S of I(X) (at most 2^3 = 8 runs, see decision 5): from the database before the group, plus the
installs of X's dependency predecessors and of S (each in a fixed order, their results proven independent in the same
way), with exactly the apps of S and the dependency predecessors installed in X's view. The runner compares what X's
runs leave with `Baseline::install_order_differences` (amendment 7 rules, amendment 8 clock window):

- **The same in every subset:** X's install result does not depend on the order.
- **Different:** the tests that touch a table that differs are not run, with the reason of amendment 7 (apps and tables
  named). Isolated storage or number sequences that differ: what reads them is refused, as amendment 7 does today.

The footprints of all of X's subset runs count for the graph. A subset run that reveals a new input or a new conflict
adds it, and the graph is built again (as the `'grow` loop does today).

### 4. Why every order then leaves the same database

Take any install order τ that respects the dependency paths. By induction over the units in an order of "reads from":
when unit X starts in τ, every resource X's session read in its subset runs holds what it held in the subset run for
S = the inputs installed before X in τ. A writer of such a resource is either ordered with X (the same in every order),
or one of X's inputs (whose own result is the same in every order, by induction), or none: no other unordered unit
writes it, else it would be an input. X observes exactly the apps of S (meets are inputs). Install code is
deterministic given what it reads (clock values excepted, amendment 8), so X runs as in that subset run and leaves the
same writes, which decision 3 showed equal for every subset. At the end, every row, counter, sequence and storage key
has one writer among unordered units (no collisions between units), so its final value does not depend on τ. Session
state never crosses installs (fact 1).

This argument needs the footprint to be complete. Decision 1 makes every gap a conflict, never a silent pass.

### 5. Limits and cost

- At most three inputs per unit (eight subset runs), and tangles of at most three apps, as today's limit of three.
  Beyond either, the run refuses as today, and the baseline note says which limit and which apps.
- A group whose apps meet nothing installs once, as today, plus the footprint record (decision 6).
- Issue 8056: A and A2 each read from B and B2 (meets), B and B2 have no install code. 4 runs of A, 4 of A2, plus the
  first install: about 9 installs of one app each, where amendment 7 would need 24 orders of the whole group.
- The 23-app suite: if every partner install reads only from the telemetry app, 2 runs per app, about 46 app installs.
  If the telemetry subscriber writes rows of its own in every session, distinct keys do not collide; an AutoIncrement
  log table or a number sequence would tie all of them into one unit and still refuse (open question 1).
- `--timings` gets one line per group: units, tangles, subset runs, and the outcome.

### 6. Where it is recorded and where it applies

- The footprint is recorded in `al-vm` at the sites that already note the impact trace and the trial reads
  (`records/op_inner.rs`, `Session::note_table_op`, the isolated storage and number sequence natives, the app-info
  natives), only while an install of a partner group runs. Off otherwise.
- It applies where amendment 7 applies: partner apps of one install group, in the dependency baseline and for the apps
  under test. Microsoft's apps and the corpus run (listed order) are not touched.
- **Main's single install of a group without meets also becomes a proof.** Today, two unordered partner apps whose
  install code meets nothing install once, and a read by one of a table the other wrote is not detected (amendment 6
  "still not pinned", amendment 7 "not covered"). With footprints it is: such a pair becomes a "reads from" edge and gets
  subset runs. This can turn a run that passes on main into a refusal where the order could really change the result.
  Migration step 3 measures it before it is turned on.

## Options considered

1. **Try every order for 4 to 5 apps.** 24 or 120 group installs, and still refuses at 23 apps. Rejected.
2. **Pairwise comparison of every pair.** Misses an effect that needs three apps in a certain order (amendment 7
   decision 3). Rejected.
3. **Treat a subscriber that changes only SingleInstance state as harmless** (the reporter's suggestion). True only when
   no later code of the install reads that state and writes from it, which needs the data flow of the session. The
   subset runs measure the result instead, which covers this shape and the ones where the state does reach a row.
4. **Let the user state the install order.** The owner rejected an order flag (2026-11-02): a pipeline installs in an
   order nobody controls. Not revisited here.
5. **Footprints only, no subset runs** (a meet as a conflict). Sound, but the 8056 shape and the telemetry shape are
   meets, so it would refuse both. Rejected.

## Consequences

- Issue 8056's repro and a group tied only through one app's subscriber run without trying every order. Correctness
  stays "the same result in every order, else refuse".
- A new recorder on every record, storage and sequence operation during partner installs. No cost outside them.
- The gap "install code reads what another unordered app's install wrote" (amendment 6, amendment 7 "not covered")
  closes. Runs that pass today only because that gap is not detected may become refusals (decision 6, measured first).
- Upgrade-tag shapes where every app's `SetAllUpgradeTags` writes every other app's tag rows collide on the same rows
  and tie the apps into one unit. Where that unit has more than three apps it still refuses. A later rule for writes of
  identical values may lift that; not part of this ADR.
- Amendment 8 item 1 (`order_trial.rs`, 289 lines) may become redundant: a subscriber that changes nothing gives the
  same result in both subset runs. Step 4 measures and deletes it if so.

### Open questions

1. What makes the telemetry app's `OnAfterLogin` subscriber fail the watched run of amendment 8 (`--verbose` gives the
   reason), and does it write rows, an AutoIncrement table or a number sequence in each install? This decides whether
   the 23-app suite is unblocked by this ADR or needs the follow-up on identical writes. To ask the reporter on 6924.
2. The limit of three inputs per unit is today's limit of three apps carried over. A higher one (2^4 = 16 runs) is a
   cost question; I propose to keep three until a suite shows a unit with four inputs.

## Migration

Each step is a draft PR, red/green, neighbor apps under `tests/neighbors/install-order-footprint/`.

1. **Footprint recorder** in `al-vm` with Rust tests per resource kind, including the "touches everything" kinds. No
   change of outcome.
2. **Influence graph and subset runs** in `al-testrun/src/install_orders.rs`, replacing `MAX_INTERACTING` for meets.
   Red first: the 8056 repro as a neighbor app (refused on main, passes), plus neighbors for each refusal: a collision
   of four apps on one row, a cycle of four apps, a unit with four inputs, a subset run that writes a different row
   (refused with the table named), an AutoIncrement log written in every session, an unattributable operation.
3. **Footprints for groups without meets** (decision 6), measured on the outside suites and NP Retail first; the owner
   decides from the counts.
4. **Measure amendment 8 item 1 against the subset runs**, delete it when nothing changes.

## Conformance

- `al-testrun/tests/suite/install_partner_order.rs` and `install_order_independent.rs` keep their outcomes (amendments 7
  and 8).
- Neighbor apps of step 2, run by `tools/neighbor-diff.sh` on BC 27, 28 and 29.
- The corpus run is not affected (listed order, amendment 6 item 3): `--baseline-diff` on BC 27, 28 and 29 shows no
  change.
