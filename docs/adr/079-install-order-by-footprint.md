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
   first** (corpus `tests/al-language/install-upgrade/install-triggers/Y79InstallUnrelated.al:142-143`,
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
HTTP, `StartSession`, `TaskScheduler` (but `CanCreateTask`, amendment 2), a Job Queue run, the Company table or a company copy, a resource the footprint
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

**Clarification (2026-11-23, orchestrator, inside the accepted decision):** a subset run installs, from the database
before the group, the dependency closure of X and its inputs in one fixed order: S, then X, then the inputs not in S.
It compares the whole end databases with the existing amendment 7 and 8 comparison (`Baseline::install_order_differences`),
not only what X leaves. That compares a superset of X's writes, so it is never less strict than the rule above and adds
no second comparator; it can only add refusals.

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

### Clarifications of 2026-11-23 (review round 1 of the first pull request)

These state what the code does where the text above is silent or shorter. They change no decision; each one is at
least as strict as the text it clarifies.

- **Decision 1, the recorder's table.** `Insert` and `Delete` write the row, not the extent. An extent read (`Find*`,
  `Next`, `Count`, `IsEmpty`, `DeleteAll`) sees every row write of its table, so a filtered read still conflicts with
  every insert, delete and modify of that table (al-data `memory/footprint.rs`, `Resource::read_sees_write`).
- **Decision 1, kept builtins.** A builtin counts as attributable only when it is on an allowlist by receiver or, for
  `System`, `Database`, `Blob` and `Query`, by method (amendment 2 adds `TaskScheduler.CanCreateTask`). Every other method touches everything, the eight encryption key
  methods among them. Between two subset runs the runner also puts the encryption key and the encrypted values back.
- **Decision 1, ties need an install session.** "Conflicts with every other session" means every other app with an
  install session of its own. An app without install code is tied only through a meet. `NavApp.IsInstalling` is not
  pinned by the corpus, so it stays "touches everything".
- **Decision 3, the run set.** A subset run installs, besides S, the unit and the inputs not in S, every dependency
  predecessor of these apps and the units those read from, until nothing is added. Each of them has a fixed place in
  the run's order, and the run gives every app it installs a view in which all the others have a known place. Apps
  outside the run set stay of unknown order; a meet with one is noted. The units that X reads from through others are
  installed before every subset run of X.
- **Decision 3, the run set: a place after the installing app (2026-11-23, orchestrator, review round 2).** This
  narrows "all the others have a known place" above. While app A installs, in a subset run or in the final install, an
  app G of the run with no dependency path to A counts as installed when it is placed before A. When it is placed after
  A, it counts as not installed only when the round's graph already has the edge "A reads from G". Then G is an input
  of A's unit or in its tangle, and A's own unit proof runs A with G installed first. Every other G stays of unknown
  order for A, so a meet with it is noted and adds the edge. Reason: G can be in the run only because a dependency
  predecessor of A reads from it, and the plan may place it after A in every run. Then A never runs with G installed
  first, and a meet of A with G is never seen. In the measured shape A's row held EMPTY where BC gives SEEN when G
  installs before A (neighbor app `install-order-footprint-hidden-meet`). The chain shape keeps its known place: the
  edge from the dependency predecessor to the app it meets is in the graph after the first run.
- **Decision 3, the run set: an order the dependencies rule out (2026-11-23, orchestrator, review round 3).** A run
  counts only when its requested order can be built. For two apps with a dependency path the dependency decides, and
  the requested place is ignored. For every other pair the requested place is a hard constraint. When no app of the
  run is free to go next, the requested order cannot be built: no run is made, and the proof stops, naming the apps
  of that cycle. Reason: when the read order runs against the dependency order (A depends on P, P reads from G, G
  reads from A), the run "G before P" cannot be built. Before, the runner then took the first app by plan rank, so
  that run never happened and the comparison found nothing. The final install passes the same order to the view and
  to the run. In the measured shape P's row held SEEN where BC gives EMPTY when P installs before G (neighbor app
  `install-order-footprint-dependency-against-read`).
  Review round 4: a subset of a unit's inputs that no real order gives is skipped, not run and not counted. That is a
  subset where an input of one app installs before the unit and an input of one app it depends on installs after it:
  on BC the second installs first. Neighbor app `install-order-footprint-dependent-inputs` (a base app and its
  extension both answer an event P's install raises; main passes). Every other order the dependencies rule out still
  stops the proof (a multi-app input, an ancestor placed first, the order of the unit's own apps; the multi-app input
  is replaced by "the places of a unit's inputs" below, which places its apps one by one). The list of
  units (`Graph::units`) puts a unit after its inputs and after the units with an app one of its apps depends on; units
  with neither tie keep their order. Every order the proof builds from the list (the subset runs and the final install)
  then keeps the dependencies unless the reads run against them or the dependencies of two units cross (see "the final
  install" below). Before, the list
  order could request an app before the app it depends on with a third app between them, and the proof stopped
  (neighbor app `install-order-footprint-final-order`; main passes).
- **Decision 3, the places of a unit's inputs (2026-11-23, issue 8907).** This replaces "S, then X, then the inputs
  not in S" for a tangle and for a unit of one app with an input of several apps, and the skip of review round 4. A
  unit's runs are its orders with each input placed among the
  unit's apps, one app at a time (the apps of an input of several apps can install on both sides of an app of the
  unit). For one order, an input app's place counts only relative to the apps of the unit that read from it: any other
  app of the unit commutes with it (neither reads from the other and they write nothing in common, else they would be
  one unit). So each input app gets one place per count of its readers it installs after, at the earliest slot of that
  class the dependencies allow. For a unit of one app with inputs of one app each these are the subsets of decision 3;
  with an input of several apps, each of its apps gets its own place (2^(read input apps) runs, not 2^(inputs); X
  between I1 and I2 of one input: neighbor app `install-order-footprint-input-tangle-interleaved`). A class no slot
  fits holds no order BC gives and is skipped. Reason: decision 4 treats a tangle as one app, but on BC an input can install between two apps of a tangle. The input that depends on
  one tangle app installs after it (T1, I, T2; neighbor app `install-order-footprint-tangle-dependent-input`, main
  before this ADR passes, round 4 stopped), and an input two apps of the tangle read from can give each of them a
  different view (neighbor app `install-order-footprint-tangle-interleaved-input`: the four runs of the old plan agree,
  T1, I, T2 differs). The runs of one unit stay at most 3! x 2^3 = 48 (decision 5); a plan with more refuses
  (neighbor apps `install-order-footprint-tangle-too-many-runs`, `install-order-footprint-input-tangles-too-many-runs`;
  main passes both: lost passes, follow-up issue 8916).
- **Decision 3, the final install: crossing dependencies (2026-11-23, issue 8907, review round 5 L15).** When the
  units' order cannot be built (two tangles whose dependencies cross: 1 depends on 3 and 4 on 2, while 1, 2 and 3, 4
  are tangles), the final install takes the plan order with the dependencies kept. Once every unit is proven, every
  order BC can give leaves the same database (decision 4), so any of them is a final install. The run set also takes
  every app of a unit one of its apps is in: before, proving 1, 2 installed 3 (1 depends on it) without 4, so 3's meet
  of 4 was of unknown order and the proof stopped (neighbor app `install-order-footprint-crossing-tangles`).
- **Decision 3, when the proof ends.** A run that adds an edge starts the proof again on the grown graph. Footprints
  only grow, so the rounds end. A run that meets an app of unknown order and adds no edge stops the proof: the group is
  not proven, and the note names that pair.
- **Decision 3, the final install.** After every unit is proven, the whole group installs once in an order of the
  units, every app at a fixed place, with the view rule of the run set above. Its footprints are checked like those of a subset run.
- **Decision 3, a group the proof cannot cover.** The install runs once more with the order not known. Refused besides:
  every difference measured before the proof stopped, and the tables written by every install that reads what another
  unordered install wrote or writes what another writes (every table when one of them touches everything or writes a
  number sequence or isolated storage).
- **Decision 4, differences follow the footprints.** A table that differs between two orders is not the end of it: an
  install that read it may have written other rows from it. The runner closes the differing tables over the
  footprints. App R is tainted by a differing table t when R's install read a resource of t (a row, its rows, its
  AutoIncrement counter, the row version, or everything) and an install of another app W of the group wrote t, unless
  W depends on R. The tables a tainted R wrote then differ too (every table when R touches everything or writes a
  number sequence, isolated storage or the row version), until nothing is added. Exempt: the apps of the unit whose
  runs measured the difference. Their own writes were compared directly in every one of those runs, so a difference
  they could pass on is already in the compared tables.
- **Facts, citation.** Fact 3 cites the asserting lines `Y79InstallUnrelated.al:142-143` (before: the header comment).
- **Conformance.** The neighbor apps run on BC 27, 28 and 29; the pull request shows the table of each.

### Amendment 1 (Accepted 2026-11-25, al-runner issue 8954): .NET members that keep nothing

Status: Accepted by the owner on 2026-11-25.

Decision 1 counts every .NET call as touching everything. Outside reporters' suites then tie four to 25 apps through
installs whose .NET calls cannot reach a lasting resource, and the whole suite is refused. This amendment narrows the
.NET row of decision 1 to two exact lists; every other .NET call keeps touching everything.

1. **A pure framework member listed in `inproc.txt`** (al-host, ADR-038 section 1), whichever host serves it, matched
   by its exact CLR type and member name (case-insensitive member), the constructor as `.ctor`. A member called on
   an object whose runtime type is a subtype the list names by its base is matched as that base's member
   (`inproc::listed_type`): a one-dimensional array type (`System.Byte[]`, any `T[]`) as `System.Array`, the encodings
   `Encoding.UTF8`, `ASCII` and `Unicode` answer as `System.Text.Encoding`, the algorithms `HashAlgorithm.Create` answers
   as `HashAlgorithm`. The match is by the listed type that declares the member; no member is added to the list
   (wording change accepted by the owner on 2026-11-25). Such a member reads and writes only .NET
   objects of the session. By fact 1 no session state, and so no .NET object in a local, a global or a SingleInstance
   global, carries from one app's install to another's. What install code takes from such an object reaches a lasting
   resource only through code the recorder sees (a record write, isolated storage, ...).
2. **A server class member whose answer here is a constant or the host identity** (al-vm `server_classes.rs`
   `ANSWERS_ONLY` and `CONSTANT_ANSWERS`): the environment, server setting and tenant getters that System Application's
   Environment Information, Server Setting and Tenant Information wrappers call, and the credential-type and Azure
   getters of `NavUserAccountHelper` that Identity Management calls. Each answer is pinned by its own
   registry citation; being a constant, it is the same in every install order. `TryGetStringTenantSetting` writes the
   answer into its `var` argument, a variable of the caller, nothing else. A setter of the same classes
   (`SetTestTenantEnvironmentType`) is not listed and touches everything.

A .NET call answered before its target is known (a `[RunOnClient]` null receiver, a cleared object, a variable never
created, a target that cannot be resolved) keeps touching everything.

**Recorder fix in the same pull request (not a change of the decision).** The safety net of step 1 compared a table the
store before the session held no entry for with no rows; the session's first write copies the test data's rows of that
table in (`table_mut`), so every such row looked like a write nobody recorded and the install touched everything. It
now compares with the rows the store before the session read for it (`row_changes.rs`, `BeforeRows::Backing`, passed by
the safety net only; the login image keeps taking such a table whole). The refusal of a unit larger
than three apps names, per reason, the call site, the install that reached it and the apps.

### Amendment 2 (Accepted 2026-11-25, al-runner issues 9082 and 9046): `TaskScheduler.CanCreateTask` keeps nothing

Status: Accepted by the owner on 2026-11-25 (asked as "Yes, after install cell"). This amendment loosens decision 1:
one TaskScheduler method no longer touches everything.

Decision 1 counts every builtin outside the kept receivers and methods as touching everything. A native builtin whose
answer is a constant for each kind of session and phase (an app's install session: false while it opens the company,
true in its install code; the test session: false) and that changes nothing keeps nothing: every install session of
every app gets the same answers, so they cannot depend on install order. Such a builtin is kept by method
(`install_footprint.rs` `KEPT_METHODS`) only when a corpus case measures its answer inside install code, cited at the
entry. Measured in a test session only is not enough.

The one builtin kept by this rule: `TaskScheduler.CanCreateTask()`. Corpus PR 1589
(`install-upgrade/IsInstallSessionTests.al:86`, `:91`, `:135`) measures true in Extension Triggers install
subscribers, per company and per database; the test session's false is pinned
(`session-system/TestEnvironmentUserTenantContracts.al:255`). The exception is the company-open phase of an install
session (the Company Triggers subscribers and the login they raise): there the runner keeps false, not measured (al-runner
issue 9123). The triggers of a `Subtype = Install` codeunit get the true answer through the same assumption, from one
install session for both kinds (`record/BfInstallTests.al:142`). Until the runner's corpus pin holds PR 1589 and the
trigger cell, the true answer runs through the assumption `install-session-can-create-task`. The other TaskScheduler
methods (CreateTask, TaskExists, SetTaskReady, CancelTask) keep touching everything.

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
