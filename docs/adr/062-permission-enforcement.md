# ADR-062: Permission enforcement: the session's permission sets, object grants and inherent permissions decide every access, in stages, refused where a denial is not pinned

- Status: Accepted (owner, 2026-10-31)
- Date: 2026-10-31
- Related: ADR-005 (access context on every data operation), ADR-007 (corpus is the oracle), ADR-009 (match BC where
  observable), ADR-013 (Microsoft's apps from their real source), ADR-027 (scope; section 2 "Permissions", owner
  decision 2, open decisions 3 and 15), ADR-031 (VM speed budget), ADR-060 (license-backed permissions; this ADR is the
  "later phase" of its section 6)

## Context

### What the runtime does today

- Every access is granted. The store's `AccessContext` holds company and user only; the permissions field ADR-005
  named was never built (`al-data/src/lib.rs:67`).
- `PermissionTestHelper.AddEffectivePermissionSet` (the platform class behind codeunit 131006 "Permissions Mock") is
  refused: a test that lowers its own permissions is not run (`al-vm/src/dotnet/server_classes.rs`
  `permission_helper_add_set`). Two exceptions:
  - the sets BC's test runner assigns before each test (`runner_permission_step`) are assumed to grant everything
    (assumption `runner-permission-sets-grant-all`);
  - `--ignore-permission-mock` runs the test with every access granted (assumption `permissions-not-enforced`).
- `ReadPermission` and `WritePermission` are always true (`al-vm/src/recmeta.rs:596`).
- Permission set objects are already read from source: Metadata Permission Set, Permission Set, Aggregate Permission
  Set, Metadata Permission and Expanded Permission (`al-vm/src/platform/permissions.rs`). `IncludedPermissionSets` is
  expanded. Wildcard grants (`tabledata * = RIMD`), `ExcludedPermissionSets` and tenant sets are refused.
- The license is read (ADR-060 phase 1). The Cronus license of the BC artifacts grants all five permissions directly on
  every object below the system range, so in the corpus CI and in Microsoft's CI it never denies anything.
- A few permission errors that do not come from permission sets are already raised and pinned: `TestPage.OpenNew` on a
  page that cannot insert (`pages/draft.rs`, `CfPageLeads.al:169`), writes in a page background task
  (`pages/background.rs`).

### What this costs (2026-10-31 gap queue, `decision table` row 1)

- **2,339 tests are not run** because they lower their permissions: 2,155 Microsoft tests (BC 29 run of all buckets;
  most in Tests-SMB 451, Tests-ERM 328, Tests-ERM-Finance 217, Tests-Fixed_Asset 187, Tests-Bank 126), 70 corpus tests
  per major, and 116 more through a cascade.
- **14 corpus test-library cases fail** (`al-language-testlibraries/lt-bi-testpermissions`): with Tests-TestLibraries
  installed, BC lowers every test whose TestPermissions is not Disabled to D365 Full Access, and the runtime grants
  everything (BiTpDefault, BiTpRestr, BiTpNonRestr, BiTpDisMRestr, BiTpDisMNonRestr: `_Permissions` and `_Insert`;
  BiInherentPerms: InsertPlain, InherentR, ScopeEntitlements, PermissionFunctions).
- **3,568 Microsoft tests pass today only under `runner-permission-sets-grant-all`** (110 more fail). These are the
  tests that could start to fail when the runner sets are enforced.
- Outside issue 6296: `Permissions Mock.Set('SUPER')` is refused, though SUPER narrows nothing. 273 and 38 tests of two
  closed-source suites are not run for this reason only.

### How a test lowers its permissions (Microsoft's source)

- `Permissions Mock` (131006, AL, compiled from source per ADR-013): `Start` creates the `PermissionTestHelper`;
  `Assign(RoleId)` does `"Permission Set".Get(RoleId)` and then `AddEffectivePermissionSet(RoleId)`; `Set(RoleId)` is
  `Clear` + `Assign('All Objects')` + `Assign(RoleId)`; `SetExactPermissionSet(RoleId)` is `Clear` + `Assign(RoleId)`;
  `ClearAssignments` is `Clear`; `Stop` is `Dispose`. "All Objects" (131006) grants `X` on every table, report,
  codeunit, page, xmlport and query by wildcard.
- `Library - Lower Permissions` (132217) and its `SetXxx`/`AddXxx` procedures push Base Application sets (D365 BASIC,
  D365 BANKING, ...) through the mock.
- The test runner (Tests-TestLibraries 130302 "Set State Test Run" -> 132218 "Permission Test Catalog"): a method whose
  TestPermissions is Disabled starts the mock with no set (the user's own permissions); every other value starts it
  with `Set('D365 Full Access')`, then `Assign` of TEST TABLES, D365 BASIC and LOCAL where they exist
  (`QcRunnerSets.al:4-24`). After the test it stops the mock.
- So the runtime needs only four members of the platform class: constructor, `Clear`, `Dispose`,
  `AddEffectivePermissionSet`. Everything else is Microsoft's AL.

### What the corpus pins (citations are `tests/al-language/` unless the path says otherwise)

**The model.**

1. With no set assigned (mock not started, started with none, or `ClearAssignments`), the user's own permissions
   apply, and the test user is SUPER: everything is allowed (`permissions/TestPermissionsMockEnforcement.al:74-97`,
   `QcRunnerSets.al:299-317`, `session/TestSessionCompanyInfoExperienceClose.al:62-90`).
2. With sets assigned, the session has exactly the union of those sets: an access they do not grant raises the
   permission error, one they grant goes through (`TestPermissionsMockEnforcement.al:28-72`,
   `lt-permissions-mock-indirect/TestMockBasics.al:20-79`). The runner sets grant what D365 Full Access, D365 Basic and
   LOCAL name plus Execute on everything, nothing else (`QcRunnerSets.al:14-24, 267-294`).
3. `Set` of a set that does not exist fails in Microsoft's AL (`"Permission Set".Get`: `The Permission Set does not
   exist. ...`, DB:RecordNotFound) and leaves the session with All Objects only (`TestMockBasics.al:143-163`). A tenant
   permission set cannot be assigned this way (`lt-av-permission-filter/AvSetPermissionFilter.al`, header).
4. **Object grants (`Permissions` property).** An object's `Permissions` property lets the code of that object use an
   indirect grant of the user. It applies to code of that object only, not to the objects it calls, and not to
   subscribers of its events:
   - codeunit with `i`, user with nothing: `IndirectInsert` error; codeunit without the property: `Insert` error
     (`lt-permissions-mock-indirect/TestIndirect.al:80-103`);
   - codeunit with `rim`, user with `rim`: Insert, Modify and Count go through; a direct Insert or Modify from the test
     fails with `Insert`/`Modify`; a codeunit with only `r` fails the Insert with `Insert`
     (`lt-cp-permission-leads/CpPermLeads.al:65-117`);
   - a subscriber with the property reads; a codeunit it calls without the property fails with `Read`; one with the
     property reads (`CpPermLeads.al:119-153`);
   - the publisher table's `Permissions` property does not cover its subscriber; the subscriber's property does when
     the user has the indirect grant (`lt-co-permission-leads/CoPermissionLeads.al:317-523`);
   - the same rule for a page action (`RiskyPagePermissions.al:107-158`, code `TestWrapped:Permission` out of `Invoke`)
     and a report (`RiskyReportQueryXmlport.al:101-138`).
5. **Inherent permissions.**
   - Table property `InherentPermissions = RIMD` grants; `= R` grants read only; `InherentEntitlements` alone grants
     nothing; both together grant (`RiskyInherentPerms.al:53-122`, `RiskyInherentEntitlements.al:45-85`).
   - The `[InherentPermissions]` attribute on a procedure grants for that call and every call made from it, and ends
     when the procedure returns; scope Permissions and Both grant, Entitlements does not; `ReadPermission` and
     `WritePermission` inside it report it (`CpPermLeads.al:155-189`,
     `al-language-testlibraries/lt-bi-testpermissions/BiInherentPerms.al:4-12, 172-288`).
   - `RequiredPermissions` on an external business event has no run-time effect (`BiInherentPerms.al:294-306`).
6. **FlowFields.** `CalcFields` of a FlowField whose CalcFormula table the user cannot read fails with an
   `IndirectRead` error (code DB:ClientReadDenied), also inside `if`; a user with `r` on that table gets the value
   (`CpPermLeads.al:29-63`).
7. **A read of a table with no rows raises no permission error** (Count, CalcFields, a plain read:
   `CpPermLeads.al:12, 47, 107, 161`). With rows, Count and FindFirst fail with `Read`
   (`CpPermLeads.al:101`, `QcRunnerSets.al:288`). `Get` of an existing row with read granted works
   (`TestMockBasics.al:33-35`).
8. `DeleteAll` and `ModifyAll` under a read-only set raise nothing **on an empty table**; with rows they are not pinned
   (`TestMockBasics.al:47-50, 70-71`; the earlier `asserterror` rolled the seed back, `:65`).
9. **ExcludedPermissionSets** subtracts per permission: Full (RIMD) excluding a set with `R` keeps Insert and loses Read
   (`CpPermLeads.al:191-212`).
10. **Wildcards.** `page * = X` is one Metadata Permission row with object id 0 (`CoPermissionLeads.al:461-477`).
11. **TestPermissions** with Tests-TestLibraries: Default, Restrictive and NonRestrictive give the same D365 Full Access
    session; Disabled (on the codeunit or the method) gives the full session; the method attribute wins
    (`lt-bi-testpermissions/BiTp*.al`). Without Tests-TestLibraries nothing is lowered (the `lt-ba-testpermissions`
    variant is a Linux suspect and not asserted, `BaTpDefault.al:4-8`).
12. **AccessByPermission**: under a read-only set, a guarded field or action is not on the TestPage (`The field with ID
    = n is not found on the page.`) (`CoPermissionLeads.al:578-599`).
13. `ReadPermission`/`WritePermission`: Tenant Media under the runner sets reads Yes/No (only indirect write); a table
    no set names reads No/No (`QcRunnerSets.al:250-287`).

**Error texts and codes.** One text shape is pinned:

`Sorry, the current permissions prevented the action. (<Kind> <Id> <Name> <Operation>: <App name>)`

| Part | Pinned values | Where |
|---|---|---|
| Kind | `TableData`; `Page` (page insert, not from sets); `System` | `TestMockBasics.al:72`, `CfPageLeads.al:169`, `al-language-onprem/lt-system-database-onprem/PCheckLicenseFile.al:37` |
| Operation | `Read`, `Insert`, `Modify`, `Delete`, `IndirectRead`, `IndirectInsert`, `IndirectModify`; `Execute` (System only) | `TestMockBasics.al:72-78`, `CpPermLeads.al:31, 115`, `CoPermissionLeads.al:356` |
| App name | the app whose code makes the access: the test app for its own and system tables, `Base Application` when Base Application code reads Company Information | `QcRunnerSets.al:255`, `report/lt-u-report-props/UPropertyTests.al:186` |
| Code | `DB:ClientReadDenied`, `DB:ClientInsertDenied`, `DB:ClientModifyDenied`, `DB:ClientDeleteDenied`; `DB:ExecuteObjDenied` (System); `TestWrapped:Permission` from a TestPage action | `TestMockBasics.al:73-79`, `PCheckLicenseFile.al:37`, `RiskyPagePermissions.al:129` |

The operation word is `Indirect<Op>` when the object whose code runs grants `<Op>` in its `Permissions` property (the
user lacks even the indirect grant), and `<Op>` otherwise. A FlowField calculation counts as an indirect read.

**Not pinned** (needs probes, section 6): `IndirectDelete`; Rename; DeleteAll/ModifyAll with rows; RecordRef writes;
temporary records; Get of a missing key, IsEmpty, Find, FindSet, Next and CalcSums without read; reads by pages,
reports, queries and xmlports; Execute on any object kind except System (Codeunit.Run, Report.Run, Page.Run, XmlPort,
Query; never observed because `Set` always adds All Objects); the app name when one app's code touches another app's
table; whether a mock assignment survives into the next test method and through an `asserterror` rollback; role ids
declared by two apps; permission set extensions at run time; non-assignable sets assigned through the mock; any
license-caused denial (ADR-060 decision 6).

## Decision

### 1. What is enforced

**Option A: only TableData writes under test-lowered sets.** Small, all pinned, but leaves reads, Execute and the
runner sets granting everything.

**Option B: the whole model, with a per-access-kind gate.** One decision function for every access; each access kind
is switched on only when its denial shape is pinned (section 5).

**Option C: model only SUPER (issue 6296) and keep refusing the rest.** Fixes the outside issue; does not move the
2,339 tests.

**Recommendation: B.** The decision function, for an access with operation `p` on object `(kind, id)`, made by the code
of object `O` with the active inherent grants `H`:

1. **License level** `L` (ADR-060): Yes, Indirect or blank. A license denial stays refused until pinned (ADR-060
   decision 6). With the Cronus license `L` is Yes everywhere below the system range.
2. **Session level** `U`:
   - no set assigned: the user's own permissions. The test user is SUPER: `U = Yes` for everything (pin 1). Sets
     assigned to the user through Access Control other than SUPER, and other users, stay refused (ADR-027 open
     decision 15).
   - sets assigned through `PermissionTestHelper`: the best level any assigned set gives after composition (section 3),
     merged with the table's `InherentPermissions` (pin 5) and the active `[InherentPermissions]` attributes in `H`
     (scope Permissions or Both). `InherentEntitlements` and scope Entitlements add nothing (pin 5).
3. **Object grant** `G`: whether `O`'s own `Permissions` property names `p` on `(kind, id)` (any level). `O` is the
   object of the current frame only (pin 4). A FlowField calculation always counts as `G` held (pin 6).
4. **Result:** allowed when `L` allows and `U = Yes`, or when `U` is Yes or Indirect and `G` holds. Otherwise the
   pinned error: operation word `Indirect<Op>` when `G` holds, `<Op>` when not; app name = the app of `O`.

SUPER needs no special case: SUPER expands from System Application's source to `tabledata * = RIMD` plus Execute on
everything, so a session whose sets include it has `U = Yes` everywhere. When the composed sets of a session grant
every object at Yes (SUPER among them), the session is marked **unrestricted** and checks nothing; this closes issue
6296 without a flag and without guessing (pin 2 gives the union; the source gives SUPER's content).

### 2. Which access points check

| Access kind | Points | Pinned denial | Stage |
|---|---|---|---|
| TableData Insert | `Insert`, `Insert(true)`, a trigger's insert | yes (`Insert`, `IndirectInsert`) | 1 |
| TableData Modify | `Modify`, `Modify(true)` | yes (`Modify`, `IndirectModify`) | 1 |
| TableData Delete | `Delete`, `Delete(true)` | yes (`Delete`); `IndirectDelete` text not pinned | 1 (direct), 1 after probe (indirect) |
| Rename, ModifyAll, DeleteAll with rows | | no | 1 after probes P1, P2 |
| RecordRef writes, temporary records | | no | 1 after probe P6 |
| TableData Read | Count, FindFirst on a table with rows | yes (`Read`) | 2 |
| Read, other shapes | Get, Find, FindSet/Next, IsEmpty, CalcSums, SetAutoCalcFields, page/report/query/xmlport reads | no | 2 after probes P14-P18 |
| FlowField | CalcFields, FieldRef.CalcField | yes (`IndirectRead`) | 2 |
| ReadPermission / WritePermission | | partly (pins 5, 13) | 2 after probe P19 |
| Execute | Codeunit, Report, Page, XmlPort, Query, Table | only System (`DB:ExecuteObjDenied`) | 3 after probe P20 |
| AccessByPermission on page controls | | one shape (pin 12) | 3 after probe P21 |
| Effective-permission queries (`GetEffectivePermissionForObject`) | | runner sets and SUPER on TableData | 3 after probe P22 |
| Runner sets (Tests-TestLibraries) | every test whose TestPermissions is not Disabled | yes (pin 11) | 4 |
| License level with `--license` | | no | 5 (ADR-060) |

**The safety rule for every stage:** an access the decision function allows needs no probe (a direct or pinned
indirect grant allows on BC as well). A **denial** is raised only in a pinned shape; a denial in an unpinned shape is
`Signal::Unsupported` naming the access and the missing permission, so the test is not run. A guessed error text is
never raised (CLAUDE.md rule 3).

The check sits in the VM at the record, page, report and object entry points, after the store has answered where the
"no rows, no error" rule (pin 7) needs the answer. The store stays permission-free: ADR-005's `AccessContext {
permissions }` is withdrawn by this ADR, because the decision needs the call stack (`O` and `H`), which only the VM
has. This is the one seam change here.

### 3. Where each permission set's content comes from

**Option A: from source.** The permission set and permission set extension objects of every app in the run (System
Application, Base Application, Permissions Mock, test libraries, the user's apps), compiled from their real source
(ADR-013). **Option B: a table of set contents extracted once from a BC database.** Faster to start, but it would
ship Microsoft content and go stale per major.

**Recommendation: A.** The existing reader in `platform/permissions.rs` is extended:

- **Wildcards** (`tabledata * = RIMD`, `page * = X`): a per-kind default level, shown as object id 0 in Metadata
  Permission (pin 10). Needed for All Objects, SUPER (DATA) and most D365 sets.
- **IncludedPermissionSets**: already expanded; level merge is "Yes beats Indirect beats blank".
- **ExcludedPermissionSets**: subtracted per permission after the includes (pin 9); nested cases after probe P11.
- **Permission set extensions** (about 720 files in Microsoft's apps): their `Permissions` and
  `IncludedPermissionSets` merge into the extended set, for the apps in the run, after probe P10.
- **Role id lookup** for `AddEffectivePermissionSet(RoleId)`: by upper-cased Role ID among the sets of the run; a
  Role ID that two apps declare is refused until probe P9.
- **Tenant permission sets** stay out: the mock cannot assign them (pin 3); Expanded Permission keeps refusing them.
- Expanded Permission and Metadata Permission read the same composed sets, so a test that reads them and a test that
  is checked against them agree.

Composition runs once per distinct list of sets and is cached in the engine, shared by every test of the run (the
runner assigns the same five sets before thousands of tests).

### 4. Performance

- **Unrestricted session (the default, SUPER, a test that does not lower): one branch per access.** The session holds
  `Option<Rc<EffectivePermissions>>`; `None` means unrestricted. No hash, no stack walk.
- **Restricted session:** `EffectivePermissions` holds, per object kind, a wildcard level and a sorted `Vec<(u32 id,
  u16 levels)>` (5 permissions x 2 bits), looked up by binary search; for TableData of the run's own tables a dense
  array indexed by the engine's table index, built on first use. Built once per set list and cached in the engine
  (`HashMap<sorted set ids, Rc<EffectivePermissions>>`).
- **Object grants:** each object's `Permissions` property is parsed at link time into the same packed form; the frame
  already holds its object index, so `G` is one lookup, and only after `U` was not Yes.
- **Inherent attributes:** a routine with `[InherentPermissions]` pushes its grants on a small session stack on entry
  and pops them on return; routines without the attribute pay nothing. Table `InherentPermissions` is a field of the
  table definition.
- **Reads:** checked once per statement that returns a row (or a count above zero), not per row of a `Next` loop.
- Budget check before each stage merges: the perf budgets of CLAUDE.md (full corpus run, the VM test) must not move
  for an unrestricted run, and `--timings` on Tests-SMB (the bucket with the most lowered tests) is recorded in the PR.

### 5. Rollout

**Option A: an opt-in flag for everything** (ADR-027 owner decision 2). No test changes outcome unless asked; the 2,339
tests stay "not run" for every user who does not pass it.

**Option B: default on at once.** Risks false failures in the 3,568 runner-set passes on day one.

**Option C: staged, default on per stage once its gate is met.** Recommended.

Why C is safe: stages 1 to 3 change only tests that lower their own permissions, and those are **not run today**. A
test that passes today cannot start failing in those stages. Only stage 4 (the runner sets) touches tests that pass
today.

| Stage | What turns on | Tests affected | Gate before default on |
|---|---|---|---|
| 0 | Composition (wildcards, excluded, extensions); unrestricted marking; `Set('SUPER')` and any all-granting set list run as normal passes; ADR-060 license stays as is | issue 6296 suites; nothing else changes | Expanded Permission and Metadata Permission corpus cases still pass; probe P0 |
| 1 | TableData writes under test-lowered sets | the 2,339 not-run tests (partly) | probes P1-P13 merged; zero denials in an unpinned shape raised as errors |
| 2 | TableData reads, FlowFields, ReadPermission/WritePermission | same | probes P14-P19 |
| 3 | Execute, AccessByPermission, effective-permission queries | same, plus `SetExactPermissionSet` users | probes P20-P22 |
| 4 | Runner sets enforced; `runner-permission-sets-grant-all` retired | the 3,568 + 110 runner-set tests; 14 corpus test-library cases | probes P23-P25, then one **shadow run** (below) with zero unexplained differences |
| 5 | License level from `--license` | partner runs with a customer license | ADR-060 decision 6 probes (restricted license in the corpus CI) |

- **Shadow run (stage 4 gate):** the runner evaluates every check under the runner sets but grants anyway, and records
  "would deny" with the test, object, operation and which input denied. Every Microsoft test that passes in
  Microsoft's CI and would be denied is a composition or model bug, fixed before default on. A test that expects the
  denial (`asserterror` + the permission text) is the only legitimate "would deny".
- **Escape hatch:** `--permissions off` (serve: `permissions: "off"`) restores today's behavior for a run, and
  `--ignore-permission-mock` keeps its meaning (the sets a test assigns are ignored; the license and the runner
  sets still apply once their stages are on). Both are reported in the run summary.
- **Flagged passes:** a check that relies on an unprobed but Microsoft-required behavior (rule 3 exception) registers
  an assumption id, and the test is a flagged pass, as for other assumptions.
- **Reporting a denial:** BC's text unchanged, plus the diagnostic line ADR-060 section 6 describes (which input denied:
  license, sets, missing `Permissions` property).

### 6. Probes the corpus needs, by stage

Shape for all: the `CpPermLeads.al` pattern (seed rows, `Permissions Mock.Start` + `Set` or `SetExactPermissionSet`,
one operation inside `asserterror` ending with an own `Error('LT .. ok <value>')`, observe text and code, restore the
mock). Own tables and own permission sets only; `TestPermissions = Disabled` on the codeunit so the runner does not
lower first. Observe on 27, 28 and 29.

**Stage 0**

- **P0 SUPER through the mock.** `Set('SUPER')` and `SetExactPermissionSet('SUPER')`: insert into an own table, direct
  insert into G/L Entry, insert into Tenant Media, `ReadPermission`/`WritePermission` of each, and
  `"User Permissions".GetEffectivePermission` for TableData, Codeunit, Page and Report. Observe: no error, Yes
  everywhere (or the differences).

**Stage 1 (writes)**

- **P1 Rename.** Set with R; RIM; RIMD on a table with rows: `Rename`. Observe text, code, the row after.
- **P2 DeleteAll/ModifyAll with rows.** Set with R; with RI: `DeleteAll()`, `DeleteAll(true)`, `ModifyAll(f, v)`,
  `ModifyAll(f, v, true)` on 2 rows, and on a filter that matches 0 rows of a non-empty table. Observe error or not,
  rows after.
- **P3 Trigger writes.** Table A with `OnInsert`/`OnModify`/`OnDelete` writing table B; A with and without
  `Permissions = tabledata B = imd`; user set RIMD on A, `imd` on B, nothing on B: `Insert(true)`, `Modify(true)`,
  `Delete(true)` of A. Observe text, code.
- **P4 IndirectDelete and the levels.** Codeunit with `Permissions = tabledata T = d`; user with `d`, with nothing:
  `Delete`. Same for `m` with user `M` (direct) to confirm direct beats the property. Observe text, code.
- **P5 App name across apps.** Code of xapp-b reading and inserting a table of xapp-a without grant, called from a test
  in xapp-c; and a test calling xapp-b that writes its own table. Observe the `: <App name>` part.
- **P6 RecordRef and temporary.** Under a set without the grant: `RecordRef.Insert/Modify/Delete`, and the same on a
  temporary record (`Temporary = true` variable and `TableType = Temporary` table). Observe error or not.
- **P7 Frame rule for nested object code.** A codeunit with `Permissions` calling its own local procedure, a procedure
  of another codeunit through an interface, and a method of a table (table procedure) that inserts. Observe which
  frames the property covers.
- **P8 Mock state lifetime.** Test method 1 does `Start` + `Set(narrow)` and never stops; method 2 of the same codeunit
  and the first method of the next codeunit insert into the narrowed table. Also `Set` inside `asserterror`: does the
  assignment survive the rollback? Run without and with Tests-TestLibraries. Observe insert outcome, `IsStarted`.
- **P9 Role id resolution.** A role id declared in two apps (xapp-a and xapp-b) with different grants; a lower-case
  role id; a non-assignable set: `Set(...)` then an insert each grants differently. Observe which grants apply or the
  error.
- **P10 Permission set extensions.** An own `permissionsetextension` adding `tabledata T = RIMD` to an own set and to
  D365 BASIC: `Set` of each, insert into T; and the Expanded Permission rows of the extended set. Observe.
- **P11 Composition corners.** An excluded set that itself includes a set; excluding `rimd` from `RIMD`; one included
  set with `R` and another with `r` on the same table. Observe Expanded Permission rows and read/insert outcome.
- **P12 Wildcard user sets.** Own sets `tabledata * = R`, `tabledata * = r`, `tabledata * = RIMD`: read and insert an
  own table, directly and from a codeunit with `Permissions`. Observe.
- **P13 System tables under a narrow set.** `Set(own set without system grants)`: insert into Tenant Media, Isolated
  Storage (through the `IsolatedStorage` API and as a record), read AllObj, Field, Company. Observe which system
  tables are always allowed.

**Stage 2 (reads)**

- **P14 Read shapes.** Set without R, with rows and on an empty table: `Get` of an existing key and of a missing key,
  `Find('=')`, `Find('-')`, `FindLast`, `FindSet` + `Next`, `IsEmpty`, `CountApprox`, `CalcSums`, `SetLoadFields` +
  `FindFirst`, `RecordRef.FindFirst`. Observe text, code, or the RecordNotFound error.
- **P15 FlowField shapes.** `SetAutoCalcFields` + `FindFirst`; CalcFormula Lookup, Exist, Sum, Count; a FlowField read
  through a TestPage field; with `r` and with nothing on the source table.
- **P16 Queries and xmlports.** A set without R on the data item table (the existing P116 query test grants R): with
  and without the object's `Permissions = r`: `Query.Open/Read`, `XmlPort.Export`. Observe.
- **P17 Reports.** `Report.Run`/`SaveAs` whose data item table the set cannot read, with and without the report's
  `Permissions`; with a request page. Observe text, code, dataset.
- **P18 Pages.** `TestPage.OpenView`/`OpenEdit` on a source table without R (with rows); a field read; a `SetValue`
  that writes without M. Observe text and code (TestWrapped or DB).
- **P19 ReadPermission/WritePermission.** With `r`, `rimd`, `I`, `IM`, `IMD`, `RIMD`; inside an object whose
  `Permissions` names the table. Observe the two Booleans.

**Stage 3 (Execute and page effects)**

- **P20 Execute.** `SetExactPermissionSet(set with tabledata grants only, no All Objects)`: `Codeunit.Run`, a call on a
  codeunit variable, `Report.Run`, `Page.Run` and `TestPage.OpenView`, `XmlPort.Export`, `Query.Open`, a table
  procedure call; then the same with a codeunit whose `Permissions` has `codeunit X = x`. Observe text and code per
  kind, or no error.
- **P21 AccessByPermission.** Field `AccessByPermission = tabledata T = R` and `= I`, action `= codeunit C = X`, a
  request page field: under sets granting and not granting. Observe visible / not found.
- **P22 Effective-permission answers.** `"User Permissions".GetEffectivePermission` and
  `NavUserAccountHelper.GetEffectivePermissionForObject` for TableData, Table, Codeunit, Page, Report under a narrow
  mocked set and under no set (closes the `docs/probes-needed.md` row for non-TableData kinds).

**Stage 4 (runner sets, test-libraries CI leg)**

- **P23 TEST TABLES.** With Tests-TestLibraries installed: read and insert a table of a Microsoft test library and of
  the corpus test app under the runner sets (the "still wanted" part of `runner-permission-sets-grant-all`).
- **P24 D365 Full Access in practice.** Under the runner sets: direct insert into G/L Entry (text, code), a post
  through "Gen. Jnl.-Post Line" (allowed by its `Permissions`), a read of Company Information.
- **P25 Lowering on top of the runner.** A Restrictive and a default test calling `Library - Lower
  Permissions.SetO365Basic` then `AddO365Setup`: insert outcomes on a table each set grants.

Total: **26 probes** (P0-P25), 14 before stage 1 can be default on.

### 7. Out of scope

- Security filters and `SetPermissionFilter` (stay a no-op; tenant sets cannot be mocked).
- Entitlements and plans of a SaaS tenant (InherentEntitlements has no observable effect in tests, pin 5).
- Permissions of users other than the session user, and Access Control changes for the session user mid-run (ADR-027
  open decision 15 stays refused).
- License-caused denials until ADR-060 decision 6 is pinned.

## Consequences

- About 2,300 tests move from "not run" toward running in stages 1-3, without any test that passes today changing
  outcome.
- Issue 6296 closes in stage 0.
- Stage 4 fixes the 14 test-library corpus cases and retires `runner-permission-sets-grant-all`; it is the one stage
  that can turn passes into failures, so it waits for the shadow run.
- `permissions-not-enforced` stays as the meaning of `--ignore-permission-mock`.
- The store stays free of permissions; ADR-005's `permissions` field is withdrawn.
- Permission set composition from source becomes a correctness dependency for thousands of Microsoft tests; a
  composition bug shows up as a permission failure, which the shadow run and the diagnostic line (which input denied)
  make visible.
- 26 probes for the corpus writer, which must land before stages turn on.

## Decisions for the owner

1. **Scope of what is enforced:** the whole model behind per-access-kind gates (recommended), or only TableData writes,
   or only SUPER.
2. **Rollout:** staged, default on per stage once its gate is met, with `--permissions off` as the escape
   (recommended); or an opt-in flag for all stages (ADR-027 owner decision 2 as written).
3. **Stage 0 before probe P0:** ship the SUPER / unrestricted marking now, derived from the pinned union of sets and
   SUPER's source (recommended), or wait for P0.
4. **Unpinned denials:** refuse the test (recommended), or allow the access and flag the test.
5. **Seam:** withdraw ADR-005's `AccessContext { permissions }` and check in the VM (recommended), or carry
   permissions into the store.
6. **Stage 4 gate:** a shadow run over all Microsoft buckets with zero unexplained "would deny" (recommended), or
   probes P23-P25 alone.

## Conformance

- Unit tests of the decision function against every pinned case in section "What the corpus pins" (table-driven, one
  row per corpus citation).
- The corpus pass-rate report: the 70 mock cases per major and the 14 test-library cases move to passing at their stage;
  no baseline case regresses.
- `--top-refusals` shows no "permissions not enforced" reason once stage 3 is on; remaining permission refusals name an
  unpinned shape and a probe id.
- The perf budgets of CLAUDE.md unchanged for an unrestricted run.
- Stage 4: the shadow run report is attached to the PR that turns it on.
