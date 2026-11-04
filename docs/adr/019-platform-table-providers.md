# ADR-019: Platform table providers

- Status: Accepted (2026-10-14). Decided without owner review, under the overnight rule. The owner reviews it
  with the other lines in `docs/owner-review.md`.
- Date: 2026-10-14
- Amends: ADR-003 (additively: a table can be answered by a provider), ADR-013 decision 7 (builds it)

## Context

ADR-013 decision 7 says that a platform table (a table of the platform `System` app) takes its schema from the
real declaration and its rows from a native provider, registered by table id; that a platform table with no
provider is refused; and that a provider never replaces AL that exists. It also says the Store gets "a new
cursor source" and that this gets its own ADR before the first provider is written. This is that ADR.

Measured before the work (ADR-013 "Experiments"): loading the declarations without providers turned 160 tests
into silent failures, because a declared table with no behaviour answers empty. Refusing is better than that
(hard rule 4).

## Decision

1. **The registry is data in `al-builtins` (`platform.rs`).** `REGISTRY` lists (table id, name, provider).
   `Provider::Virtual(kind)`: the rows are computed on read and the table cannot be written.
   `Provider::Stored`: an ordinary stored table with its real declaration (the Company, the User, Record Link).
   `al-sema` reads the registry, so the refusal needs no runtime.
2. **The System app is an implicit dependency of every app** (`Workspace`, `PLATFORM_APP_ID`), loaded like any
   dependency from the project's `.alpackages`.
3. **A platform table with no registry entry is unusable** (`Workspace::usable`). Wherever code names it the
   result is the existing "outside the workspace" refusal, worded
   `platform table 'Date' (2000000007) has no provider` (`Workspace::outside_workspace`). This replaces the
   old unconditional refusal of the whole System app.
4. **The seam in `al-data`:** `VirtualTables` (serves, get, next, count) and `StoreError::Refused(String)`.
   `MemStore::with_virtual_tables` takes a provider; for every table it serves, reads go to the provider and
   every write, delete, rename and `DeleteAll` is `Refused`. Such a table is not part of a snapshot, a commit or
   a rollback. Temporary records of a platform table are ordinary temporary tables (the provider is only on the
   session store).
5. **Providers live in `al-vm/src/platform/`.** They read only the compiled `Program` and the host values of the
   session (`al-host` `Identity`): no OS access (ADR-006). Two shapes:
   - *generated* tables (Integer, Date, Windows Language) answer a scan from the key filter alone, with interval
     arithmetic (`ranges.rs`), so `Integer` over -1000000000..1000000000 costs nothing;
   - *listed* tables build their rows once per session (`mat.rs`) and scan them in key order.
6. **A row the provider cannot work out is poisoned**, not dropped or defaulted: it keeps its key and a reason.
   A read that the key filters cannot exclude it from is refused with the reason; a read that excludes it is
   answered. So one object with an unusual property refuses only the reads that could touch it.
7. **A column nothing pins is refused where code reads it.** `PINNED_COLUMNS` lists, per registered table, the
   columns a corpus test fixes. Lowering (`readable_field_pos`) refuses a routine that names any other
   non-system column of that table. The provider still writes a plausible value there, so a dynamic reader
   (FieldRef, TransferFields) sees it; the refusal is the guard for code that names the column.
8. **Objects nothing compiled still have names, ids and properties.** The program keeps a shell for every
   object of every loaded app (ADR-013). The declaration scan (`al_syntax::scan`) now also reads the properties
   written directly in an object body (`Caption`, `Subtype`, `TableType`, `PageType`, ...), checked against the
   parser on every corpus and Microsoft file (`scan_equivalence.rs`). Shells carry them, plus `namespace` and, for
   extensions, `extends_id`. Information a shell cannot give (fields, keys, controls) poisons the rows that need it.
9. **Stored platform tables are always compiled** (the demand pass raises them to READ), and the session seeds
   the rows AL reads as facts about the session when it starts (`seed.rs`): the Company (`My Company`, the
   company id of ADR-013 decision 9), the User (`TESTUSER`, the security id) and the SUPER Access Control row.
   They are written and committed, so a rollback keeps them, and AL may change or delete them.
10. **Host data comes through `Identity`:** `company_id`, `host_name`, `time_zones` are new defaulted methods.
11. **Version gates.** `SuperPermissionSetIgnoresAppId` is consumed by the permission providers (see the status
    table in `docs/status.md`).

## Decided without owner review (2026-10-13/14)

Each is also a line in `docs/owner-review.md`.

1. **Unpinned columns are refused statically, not at run time.** Options: refuse at lowering (chosen: the test
   is reported not run before it starts, hard rule 4); fill a default silently (rejected, hard rule 3); poison
   the whole row (rejected: loses the columns that are pinned).
2. **Package ids are a function of the app id** (`ids.rs`). A real tier assigns them at publish time and no
   process can know them. The corpus pins only: not empty for an app's own objects, the same for every object of
   one app, different between apps, and the same in AllObj, AllObjWithCaption and the app tables. AL Runner v2
   uses the same contract. Objects of the platform app have the empty id (AL Runner: no owner).
3. **The default host's time zones** are five Windows zones with the names Windows gives them (UTC, GMT, W. Europe,
   Eastern, Pacific). The corpus pins "more than one, `No.` runs 1 to N, a non-blank id" and says the list is the
   host's. A host can supply its own list.
4. **Windows Language holds three rows** (1031, 1033, 2057), the ones the corpus fixes. A read that would have to
   answer for any other id between 1 and 65535 is refused; an id above that is no language (999999 is pinned).
5. **A `Get` whose result is used, in a dependency app's code, on a table the install baseline could not seed is
   refused** like the raising form (`records.rs` `fail` checks `Engine::unseeded` before the quiet `false`).
   With the platform tables loaded, `install-upgrade/TestRetenPolAllowedTables_Tests.al` ran one step further and failed,
   because the retention policy list is filled by Base Application's install code.
6. **Provider reads and snapshots.** A provider holds the engine weakly and a captured baseline holds no provider
   (`MemStore::detached`), so the engine stays unshared after the baseline and `Engine::unseeded` can be set.
7. **Page Metadata:** a page type outside the thirteen the column names and PromptDialog (20) is poisoned; a page
   with no source table reports every source-object flag false (AL Runner measured this on BC); a
   `SourceTableView` is formatted only for the single `where(field = const(value))` form, with an option or enum
   member as its ordinal (`Status = const(Active)` is `CONST(2)`, TestPageMetadataSourceObject.al:61-80).
8. **Page Control Field:** how a declared `Visible`, `Enabled` or `Editable` is spelled is lower case
   (`false`); only the undeclared defaults are pinned (`true`, `true`, `True`).
9. **Date: "Period Invariant Name" equals "Period Name"**, because the culture is en-US and the declaration calls it
   the language-independent name. Listed in `docs/probes-needed.md`.
10. **All Profile is a stored table.** A session seeds one row per `profile` object of every loaded app (Scope
    Tenant, the app's id), and two native rules guard those rows (`platform/profiles.rs`): they cannot be deleted
    or renamed away, with the texts the corpus pins. Options: a virtual table over the profile objects (rejected:
    AL inserts, renames and deletes tenant rows, `profiles/TestAllProfileTable.al`).
11. **Report Layout List, Report Metadata and Report Data Items** come from the program. The scanner now keeps
    the layouts of a report's `rendering` section (also for shells), the IR keeps `RequestFilterFields`. "Data
    Item ID" is the item's position times ten: the corpus only needs the ids of two items to differ. An
    `ObsoleteState` of `Removed`, a layout `Type` other than RDLC, Word, Excel or Custom, and a view the provider
    cannot spell (more than sorting, order and one `field = const(v)`) poison the row.
12. **XmlPort Metadata and Query Metadata:** a property an object does not declare reads the documented AL default
    (Direction Both, Format Xml, UseRequestPage true, TransactionType UpdateNoLocks, TextEncoding UTF8). An API
    query without `APIVersion` is poisoned (the default is not documented).
13. **Event Subscription lists the subscribers of compiled codeunits only.** An uncompiled codeunit might declare
    subscribers, so each is one poisoned row standing for all rows of that codeunit: a read that a filter on the
    subscriber codeunit excludes is answered, an unfiltered walk or a publisher filter is refused. A database
    trigger event of a table (`OnAfterInsertEvent` and the like) has the event type Trigger; a subscriber of a
    manual-instance codeunit is poisoned (Active is not pinned).
14. **Page Action lists the compiled pages' actions.** Each `area` of `actions` is a container row (type 0, parent
    0), then actions (1) and groups (3) depth first; "Action Index" and "Action ID" both count from 1. An action
    reference, a system action or an action outside an area poisons the page. The IR page keeps `action_areas`.
15. **Table Relations Metadata lists one row per arm, and per condition of an arm that has some.** A condition
    written `Field = filter(...)` is not understood by the runtime's relation check, so the compile keeps the
    tables and fields such a relation names (`Field::relation_meta`), for this table only.
16. **A poisoned row may stand for a key prefix** (`Entry::poisoned_prefix`): where a provider cannot list the
    rows of an uncompiled object, one row with only the object's id known is refused by every read that no
    filter on that id excludes. Filters on the unknown columns never rule it out.
17. **Code Coverage, Code Coverage Test Lookup and Code Coverage Tests Run are stored and start empty.** The
    runtime records no coverage. After `CodeCoverageLog(true, ...)` every read of them is refused until an
    unfiltered `DeleteAll` empties that table (`platform/coverage.rs`). A recording engine would replace this.
18. **A platform table nothing provides is still a table (round 3).** It lowers and its shape is the declaration. A
    temporary record of it is an ordinary temporary table, as the corpus pins for Field, Date and Aggregate
    Permission Set (record/TestTemporaryVirtualTableRecord.al); a variable that is declared and never read does
    not stop the code that holds it; a read or write of its persistent rows is refused at run time with
    `platform table 'X' (id) has no provider` (`Virtual::Unprovided`, `Providers::new`). Before, the first mention
    of such a table in a codeunit refused every test that reached the codeunit (about 1,900 tests on Microsoft's buckets).
    The columns of such a table are not pinned-checked: nothing reads a persistent row.
19. **Columns the declaration describes are filled from it** where the Microsoft buckets read them: Page Metadata
    Caption and CardPageID, Field TableName, Class, Enabled and "Field Caption", Key MaintainSIFTIndex and
    ObsoleteState, Report Layout List "Application ID", Table Metadata DrillDownPageId, and Field RelationTableNo for a
    relation with a `filter(...)` condition (`Field::relation_meta`). None is pinned by a corpus test; each is a line
    in `docs/probes-needed.md`, "Probe batch 5".
20. **A refused store read prints as its reason** (`StoreError`'s `Debug`), so the ranking of refusals names the table.

## Consequences

- A platform table becomes usable by one registry entry, one provider and the corpus lines that pin it. Nothing
  else changes.
- A read the provider cannot answer is a refusal with the reason, never an empty answer.
- The program keeps the properties of every object, which costs memory proportional to the declared properties of
  Base Application (measured: no visible change in the 17 s run).
- Dependency code that reads a table the install baseline would fill now refuses more often (decision 6 above)
  until the baseline exists.

## Conformance

- `al-test-cli/tests/platform_tables.rs`: every registry entry is a table of the System app with that id and name
  and no code; no other app supplies a table under a registered id or name (ignored in debug, CI release step);
  a platform table with no provider is refused naming table and id; a virtual table can be read and not written;
  a column nothing pins is refused where code reads it.
- `al-data` `virtual_tests`: a served table reads from the provider and refuses writes; another table is stored.
- `al-vm` `platform::mat`, `ranges`, `date`, `ids` unit tests.
- `al-deps/tests/scan_equivalence.rs`: the scanned properties equal the parsed ones on every file of the corpus and
  of the Microsoft apps.
- `crates/al-sema/tests/expected-unresolved.txt` now lists exactly the platform tables with no provider.
- The corpus run is the integration test; `metrics/<major>/passing.txt` the ratchet.
