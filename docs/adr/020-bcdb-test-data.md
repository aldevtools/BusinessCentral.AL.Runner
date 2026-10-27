# ADR-020: Test data from a BC database file: `al-bcdb` and a table provider

- Status: Accepted (owner, 2026-10-14: build the loader in point 3). The loader is built (branch `test-data`): `alrunner test --test-data`.
- Date: 2026-10-14
- Owner decision (Stefan, 2026-10-14): test data is **on by default**. `alrunner test APP` loads the CRONUS demo
  company of the matching BC artifact; `--no-test-data` turns it off. `--test-data [PATH]` stays, to name a
  backup or to insist (a missing backup then stops the run). Built on branch `zero-config`.

## Context

AL Runner v2 loads the CRONUS demo company with `--test-data`. It shells out to `bcdb`, the
owner's C# reader for SQL Server backups and BC cloud exports, and rebuilds each table's rows
into its store the first time a run touches the table (`TestDataProvisioner.cs`,
`RecordPatches.TestDataHydration.cs`). Microsoft's Base App test buckets need that data: on
Tests-SMB, switching `--test-data` on more than doubled the passing count
(`running-ms-test-buckets` skill).

The runtime needs the reader in-process, fast, and usable from WebAssembly. A subprocess per
table costs a process start and a page-map build each time, and cannot run in a browser.

## Decision

1. **`al-bcdb` is a core crate** (ADR-006): a port of bcdb 0.2.1 that works on a
   `ByteSource` (a trait with `read_at`/`slice`) and does no OS access. `.bak` and `.bacpac`
   both open through `al_bcdb::open`. Its output is typed (`Value`), and its text form is
   bcdb's text byte for byte.
2. **`al-bcdb-cli` holds the OS access**: a `pread` byte source and the bcdb command line
   (`tables`, `companies`, `describe`, `read`, `serve`, `check`, `validate`, `verify`), plus a
   parallel `export`. It is a development tool and the oracle for parity tests; it is not
   wired into test runs.
3. **The loader** (built; `alrunner test --test-data [PATH]`) works like this:
   - A `TableProvider` capability trait in `al-host` (additive): given a company and a table's
     SQL name, it streams typed rows. `al-test-cli` implements it over `al-bcdb`, as it
     implements `Clock` today. The core never sees a file.
   - **On first touch of a table** in a Session, the store asks the provider and fills the
     table. A table nothing touches is never read. As built: the store holds no entry for a
     table nothing has written; reading such a table asks the session's `Backing`
     (`al_data::memory::Backing`), and the first write copies the backing's rows into the store. This is AL Runner's measured lesson: eager
     loading put 37,710 rows into the install baseline and made every test boundary cost
     105-203 ms.
   - Rows read after the install baseline was captured are added to the baseline as well, so
     the per-test reset (a root-pointer swap, ADR-004) does not drop them. As built, nothing is
     copied into a baseline: the reset swaps the store for the baseline's, which has no entry for
     a table nothing wrote, and a table without an entry reads the backing again (a clone of a
     map built once per thread). A rollback behaves the same way. The restored rows are
     committed data by construction.
   - **Names come from the compiled program, not from symbol files.** The program already
     knows each table's fields and extension fields, so a column `Name_` maps to field `Name.`
     by the same normalization (`SqlNames.Normalize`) without loading the 58 MB Base
     Application symbols. `$ext` companions are merged on the companion's own key, as
     `bcdb read --merge-extensions` does (`al_bcdb::query::plan_read_table`).
   - **Refusals are explicit**, one per AL Runner's declared exclusions: same-named tables no
     app id can select, a value type the runtime cannot rebuild, a table the reader fails on
     (reported with the reader's text, never fatal to the other tables), extension columns of
     an app outside the closure (dropped and counted), a column the AL table does not have
     (dropped and counted; refusing the table leaves it empty, which AL reads and believes),
     a row shape sharing no column with the target (refused). The summary line reports each
     count.
   - **The source is a cached Microsoft artifact outside the repository** (hard rule 7):
     `~/.bcartifacts.cache/sandbox/<version>/<country>/BusinessCentral-<country>.bak`. A
     `.bacpac` of the owner's own database is the same call with a different file.
4. **Dependencies added**: `sha2` (the `model.xml` checksum and `--sha256`), `flate2` with the
   `zlib-rs` backend (zip inflate; pure Rust, builds for WebAssembly, faster than
   `miniz_oxide`), `quick-xml`, `serde`, `serde_json` (already in the workspace) in the core;
   `rayon` in the command line only. No dependency touches the OS from the core.
5. **Fixtures**: bcdb's synthetic `typeprobe` database (8.2 MB: a `.bak`, a `.bacpac`, the
   oracle `SELECT` exports, two symbol files) is copied into `crates/al-bcdb/tests/fixtures`.
   It holds no Microsoft data. Fixtures of the demo database (`bc275-*`, `bc281-*`) are not
   copied.

## What was built (2026-10-14, branch `test-data`)

`alrunner test --test-data [PATH] [--test-data-company NAME]`. The pieces:

- `al-host::testdata`: `TableProvider` (company, source name, `read(table)`), `Cell` (the typed cell,
  one to one with `al_bcdb::Value`), `TestData` (provider, the counts, and a once-per-table read memo
  so test threads that touch a table together read it once), `LoadStats` and `Summary` (the counts
  behind the summary line).
- `al-bcdb-std` (new crate, the OS side): `FileSource` (the pread byte source `al-bcdb-cli` also uses)
  and `BackupProvider` over `al-bcdb`: picks the company, finds a table by AL name and declaring app
  id, merges `$ext` companions with `plan_read_table` (no symbols), returns SQL column names.
  `default_backup` finds `~/.bcartifacts.cache/sandbox/<build>/w1/BusinessCentral-W1.bak`.
- `al-data::memory`: a `Backing` the store reads a table from while it holds no entry for it.
- `al-vm::testdata`: `TestBacking`, the backing of a session. It maps columns to fields of the compiled
  table, rebuilds the values, builds the table once per thread and counts every exclusion.
- `Engine::test_data` (set from `RunOptions::test_data`), so every session of a run, the install
  baseline's included, reads the same test data.

### Names

A column `X` is the field whose name SQL-normalizes to `X` (`.` `"` `\` `/` `'` `%` `[` `]` become `_`)
and that the table's own app declares (the table, or a table extension of the same app: BC stores
those in the base table). A column `X$<app id>` is the field of that name declared by that app's table
extension. A column `$systemId` ... `$systemModifiedBy` is the system field, `timestamp` is
`SystemRowVersion`. The declaring app of a field is `ir::Field::declared_in` (additive IR field).
Two fields for one column, or two columns for one field, refuse the table.

### Counted exclusions (the summary line)

Every one is a count in `LoadStats`, and the summary prints a line per refused table with the reason:

| Exclusion | Effect | Counted as |
|---|---|---|
| Same-named tables no app id selects | table reads empty | `skipped (a name no app id could select)` |
| A value the runtime cannot rebuild, a row shape sharing no column, two rows with one primary key | table reads empty | `refused (a value type or row shape ...)` |
| The reader fails on the table | table reads empty; the reader's own text is printed | `refused by the backup reader` |
| Extension column of an app outside the closure | column dropped | `extension column(s) dropped for apps this run does not install` |
| Column no field of the compiled table has | column dropped | `column(s) dropped that the compiled tables have no field for` |
| Platform tables (object id from 2000000000) | the runtime keeps that state itself | `platform table(s) not loaded` |

A refused table reads as empty, as in AL Runner v2; that is the dangerous answer v2 measured on
`No. Series Line`, which is why a NULL, an empty text and an empty blob rebuild as the field's blank and
only a non-empty value of an unrebuilt type refuses.

Value types not rebuilt yet (a non-empty cell of one refuses the table, with the field named): see
`al-vm/src/testdata.rs` `Kind::Unsupported`.

### Install baseline and restored rows

The provider is armed before the install roots, as in AL Runner v2 (`TestDataProvisioner.Arm` runs in
front of the dependency install triggers, so an install trigger reads the backup's rows, the order a
restored database has). Install code must not change what the backup holds, so while a session is in
install scope the store refuses `Modify`, `Delete`, `Rename` and `DeleteAll` of any row the backing holds
(`MemStore::guard_backed_rows`). An `Insert` of a key the backup holds fails with BC's "already
exists"; an `Insert` of a new key adds a row. A refused root stops where it stops: an install trigger is
rolled back and `Company-Initialize` keeps the rows before it, both reported as before.

### Measured (2026-10-14, 27.5 recipe, `docs/alrunner-buckets.md`)

Tests-SMB: 6 passed, 6 failed, 1018 not run without test data; 10, 3 and 1017 with it (2953 rows from 97
tables, 4.3 s in all, 1826 MB). Tests-ERM: 227, 121 and 8985 without; 434, 9 and 8890 with (15352 rows
from 167 tables, 6.7 s, 2.9 GB, the same peak as without within 2 percent). No install root was refused by
the restored-row guard in either bucket. The failures that remain are listed in the buckets document and are
runtime defects, not differences between the restored company and a freshly prepared one.

## Consequences

- `--test-data` needs no .NET process and no second program on the path.
- The reader's memory use is bounded by one table's rows in flight; the page map (about 1 MB
  for the 28.1 demo backup) is the only whole-file state.
- Every behaviour of the port is held to the published bcdb by `crates/al-bcdb-cli/tests/differential.rs`
  (see `docs/bcdb.md`). A fix in bcdb needs the same fix here, and the parity tests show a gap.
- `bcdb restore` is not ported.

## Decided without owner review (2026-10-14)

- Two crates (`al-bcdb`, `al-bcdb-cli`) rather than a binary inside the core crate, so the core
  stays free of `std::fs` and threads.
- `pread` rather than a memory map in the command line: 17 ms against 37 ms to open the 28.1
  backup, because a map pays a page fault per touched extent.
- `tables` is sorted by an approximation of the culture-aware order bcdb uses; it matched bcdb on
  all four demo backups and a real `.bacpac`.
- `read` prints nothing when a row cannot be decoded, as bcdb does, by buffering the answer.
- The `typeprobe` fixtures are committed (8.2 MB of binary).

## Decided without owner review (2026-10-14, test-data)

- **Order: restored data first, install on top** (AL Runner v2's order, `TestDataProvisioner.Arm` before
  the install triggers; Microsoft's pipeline runs `Company-Initialize` first and the demo data after,
  but v2's `--test-data` presents a restored company, and a restored database has had its install code
  run already). The two alternatives were worse: install code on an empty store with the backup laid
  over it adds rows the backup lacks (the autoincrement rows of an install trigger), and skipping the
  install roots leaves tables the backup does not hold without their seeds. Install code reads the
  restored rows, so `if not Setup.Get() then Insert` finds the row.
- **(Superseded by the Accepted amendment, 2026-10-15: install code may change restored rows.)** Install code may not rewrite restored rows: in install scope the store refuses `Modify`,
  `Delete`, `Rename` and `DeleteAll` of a row the backing holds (a key test, `MemStore::guarded`). v2
  allows it. The refusal ends that root with its reason in the `baseline:` notes; an `Insert` of a
  restored key fails with BC's "already exists" and rolls the root back, so nothing is duplicated.
  Rows the install code inserted itself are its own to change.
- **The session company is the backup's company**: `CompanyName()` answers the `Name` of the restored
  company's row in the backup's `Company` table (`CRONUS International Ltd.` for the demo database,
  which equals what the runtime answered before: the corpus pins that text), so the corpus runs and the
  `--test-data` runs agree. A backup with several companies restores the demo company; any other needs
  `--test-data-company`, as v2 needs `--test-data-company`. A record in another company on a table the
  backup holds rows for is refused (the other company's rows are not modelled), as for baseline rows.
- **No baseline copy**: see point 3. A table nothing wrote reads from the backing; the first write copies
  the rows into the store. Per-codeunit reset, rollback and `Checkpoint` restore therefore need no
  special case, and a table that was never touched costs nothing at a reset.
- **Platform tables (from object id 2000000000) are not loaded** and are counted. v2 loads the Tenant
  Media family; this runtime has no Tenant Media rows to put them in.
- **A table refuses the whole table, as v2 does, but only for a non-empty value of a type it does not
  rebuild** (DateFormula tokens, RecordId, TableFilter); NULL and empty cells are the field's blank.
  DateFormula (BC's token string, `date_formula_text`) and compressed and plain Blobs are rebuilt (BC's marker and a raw Deflate stream, v2's reading of
  `NavSqlCommand.GetBlobDataFromReader`).
- **AutoIncrement continues above the restored rows** (`docs/probes-needed.md`): without it the first
  insert into a restored identity table fails on key 1. `SystemRowVersion` is restored but the counter
  is not seeded (documented gap).
- **`--test-data PATH`** takes PATH only when it ends in `.bak` or `.bacpac` (or is written
  `--test-data=PATH`): an APP is a positional argument, so a bare name after the flag is an APP.
  `--test-data-normalize-company` stays refused: its rules are v2 measurements this runtime has not
  repeated.
- **A new crate `al-bcdb-std`** holds the file byte source and the provider, so `alrunner-cli` does not
  depend on a development command line, and `al-bcdb-cli` and `alrunner` share one `pread` reader.
- **Default backup**: the cached artifact folder named for the build (`--bc-version` with four numbers),
  else the newest folder with the same `major.minor`, else the newest of the major; country `w1`, else
  `us`, else the only one.

## Decided without owner review (2026-10-14, zero-config)

- **Where the default backup comes from, in order**: the build of the Microsoft apps in
  `~/.bcartifacts.cache/sandbox/<build>/w1`; the same build in the artifact cache
  (`<cache>/artifacts/<build>/demo/w1/BusinessCentral-W1.bak`); a download of the `BusinessCentral-W1.bak` entry of the
  feed's country artifact (`<feed>/<build>/w1`, a ZIP; read by range and streamed through the inflater to a `.part` file,
  size and CRC-32 checked, then renamed; `al_deps::demo`); then a backup of another build of the same BC version, with a
  note that names both builds. The download is about 520 MB and happens once. `--no-download` forbids it.
- **A default run that finds no backup, or cannot open one, runs with test data off** and prints one `note:` line with the
  reason, so a failed download never fails the run silently and never stops it. A run that asked for test data
  (`--test-data`, `--test-data PATH`) stops with the reason instead.
- **A default run with no Microsoft app in it** (no Microsoft APP and no Microsoft dependency loaded) has no test data
  and prints nothing: the CRONUS company means nothing to it, and it must not download 520 MB for nothing.
- **The country is `w1` by default.** (Amended 2026-10-17: `--country CODE` selects another; see the amendment at the end.)
- **`--test-data-company` no longer needs `--test-data`**; it conflicts with `--no-test-data`.

## Decided without owner review (2026-10-14, npcore-fixes)

- The restored-row guard covers the dependency apps' install roots and `Company-Initialize` only. The
  install codeunits of the app under test run before each test codeunit without the guard (2026-10-14,
  npcore-fixes): they are the project's own fixture, a real project's seeds and changes the setup tables of
  the demo company (NaviPartner's `NPR Test Fixture` modifies `Sales & Receivables Setup`), and a published
  app's install code may change a database that already holds data. With the guard they were all refused
  (1466 tests not run); the dependency roots keep it because Microsoft's install code must not change what
  the backup holds.

## Decided without owner review (2026-10-14, testdata-speed)

Measured on the owner's demo app (10 tests, BC 28.5) with a cold page cache: opening the 934 MB backup took
3.4 s, of which 2.4 s were the page map (a 40-byte read of the lead block of each of 14,900 extents; on a cold
file the kernel reads about 270 MB for them) and 0.4 s the catalog (a scan of the system tables). Warm, the open
is 30 to 60 ms. Each table read also parsed and sorted the names of all 4,100 tables again (4 to 8 ms per read,
63 reads in the demo, 556 in Tests-ERM).

- **The table list is parsed once**, at open (`query::plan_read_table_in`, `extension_columns_in`). A read of a
  small table went from 4 to 8 ms to 0.2 to 0.5 ms.
- **The page map and the catalog are kept** in `<cache dir>/backup-index/<file>-<hash>.pagemap` (2 MB for the
  demo backup; the hash covers the path, the length and the modification time). A kept map is used only when the
  file length and the data regions of the container still match, and the catalog part carries a checksum; anything
  else is ignored, rebuilt and written again. The first open of a backup still pays the 2.4 s once. With the
  index, a cold open is 8 to 30 ms. `--no-compile-cache` also turns the index off. This is a second persistent
  cache (the brief allows only the compile cache); the owner asked for it by name in the task, the work it avoids
  is a verification pass over the whole file, and the data itself (the rows) is not cached. Say no to remove it.
  The catalog part also keeps the backup's SQL sequences (2026-10-27, run-speed task of the owner): every session
  of a run with test data asks for them, and walking two system tables for them cost about 30 M instructions per run.
  An index saved without them gains them on its next open and is saved once more.
- `--timings` prints one `test data:` line: open time, whether the kept index was used, number of table reads,
  rows and time.

## Amendment (Accepted, owner 2026-10-15): install code may change restored rows freely

This replaces "Install code may not rewrite restored rows" (the guard) and the exception for the app under
test's install codeunits. Owner decision, 2026-10-15: "Why not completely allowing". The model is AL Runner v2's
(`TestDataProvisioner.Arm` before the install triggers, `RecordPatches.TestDataHydration`):

1. A table is loaded from the backup the first time anything touches it, install triggers and Company-Initialize
   included (unchanged: `Backing`, one read per table per run).
2. Install code and Company-Initialize run on top of those rows and may modify, rename and delete them, and
   insert rows (an insert of a restored key still fails with BC's "already exists"). Nothing is refused.
3. The state after the install code is what every test codeunit is reset to, not the raw backup. As built, this
   needed no new code: the first write to a table copies the backing's rows into the store, so the captured
   baseline holds the table with the install code's changes; a table install code never wrote has no entry in the
   baseline and reads the backing, so a test that first touches it gets the backup's rows as the backup holds
   them. Test `tests_are_reset_to_the_state_after_install_code_not_to_the_raw_backup` pins both.

Built: the guard (`MemStore::guard_backed_rows`, `Session::allow_restored_rows_to_change`) is removed. While
install code runs, the store writes down every change to a row the backing holds (`RestoredChange`), and the
baseline prints one `baseline: install code changed restored rows of <table>: ...` line per table, with the
names of the fields whose values differ (system fields apart).

### What BC has, and why this is safe

The backup is a database in which Company-Initialize and the install triggers already ran, followed by the demo
data load; a real test run starts on it and does not run them again. Running them on top is the runtime's way to
get what the backup lacks (apps it does not know, state kept outside tables). A rewrite of a restored row is
either a no-op, a repeat of what already happened, or a replacement of a demo value by the value install code
would write; the owner accepts the latter.

### Measurement (2026-10-15, BC 27.5, `tools/bucket_diff.py` on JUnit, per test)

| Run | Passed / failed / not run | Tests with another outcome |
|---|---|---|
| Tests-ERM, main before and after | 867 / 1 / 8,465 both | 0 |
| NP Retail, main before and after | 325 / 0 / 4,488 both | 0 |
| Tests-ERM with laptop issue 29 (Option to Enum) merged into a scratch branch, before and after | 870 / 1 / 8,462 both | 0 |
| NP Retail with issue 29, before and after | 325 / 0 / 4,488 both | 0 |
| Corpus baselines 27, 28, 29 | no baseline case changes (the 51 cases that fail without being in the baseline are the same on main) | 0 |
| `demand_vs_eager`, `cache_vs_uncached`, `baseline_rule_check` | 0 outcome differences | 0 |

No test goes from pass to fail or not run. Which restored tables install code changes:

- Tests-ERM with issue 29: only **Application Area Setup**, one Modify (by `SetExperienceTier`, reached once
  Company-Initialize gets past the enum assignment). It writes the values the row holds; only `SystemRowVersion`
  differs. A rule that allowed only no-op writes (system fields apart) would have been enough for Tests-ERM.
- NP Retail: **Application Area Setup**, two Modifies, by the NPR apps' own install code. They set the NPR
  application areas (`NPR Retail`, `NPR Ticket Advanced`, ...: fields the backup does not know, so they differ).
  A no-op-only rule would have refused them. This is the case the owner's decision covers.
- Without issue 29, Company-Initialize stops at the enum assignment before any restored row is changed, so
  the Tests-ERM run shows no change at all. With the guard and issue 29, Company-Initialize stopped at
  `SetExperienceTier`; now it goes on and stops at `DotNet new ...AzureADGraphClient.GraphQuery` (602 tests).
- The pass counts changed from earlier runs (887 / 355 on the morning's main, 867 / 325 on the main used here):
  that is main moving between the runs, identical in the before and after columns above.

## Conformance

`cargo test -p al-host -p al-data -p al-bcdb-std -p alrunner-cli` and
`cargo test -p al-testrun --test test_data` hold the loader (a fake provider for the order, the reset, the
read-once guarantee and the counted exclusions; bcdb's `typeprobe` backup, as `.bak` and `.bacpac`, for
the reader end to end).

`cargo test -p al-bcdb -p al-bcdb-cli` (hermetic: oracle fixtures, framing bytes, serve and
command-line contracts). `cargo test -p al-bcdb-cli --test differential -- --ignored` compares
with the published bcdb on the fixtures, on every artifact `.bak` in the cache, and on a real
`.bacpac`. `tools/bcdb-bench.sh` measures speed.

## Amendment (Accepted, owner 2026-10-17): `--country`, flag only

`alrunner test ... --country us` (also `alrunner warm`) runs against another BC country version. The default
stays `w1`. Only the flag selects a country: it is never read from AL-Go settings or `app.json`.

### What the country selects

The country artifact `<feed>/sandbox/<build>/<country>` (measured on 28.5.54151.56408 and 24.5.23489.42600) holds:

- `BusinessCentral-<COUNTRY>.bak`, the demo database (US 28.5: 610 MB stored, 1.1 GB inflated).
- `Applications.<COUNTRY>/`: Source.zip files of the country's own Microsoft apps (Base Application, Application,
  System Application, Business Foundation, Any, Library Assert, test libraries, ...) and Microsoft's test apps
  `Tests-*.Source.zip` (the buckets), flat in one folder. W1 keeps the same apps in the platform artifact under
  `Applications/**`.
- `Extensions/` (compiled packages, not used), `ConfigurationPackages/`, `manifest.json`.

The platform artifact stays the same (`System.app`, the apps no country changes).

### Rules

1. **Apps.** For a country other than `w1`, a Microsoft app that the country artifact has a Source.zip for under
   `Applications.<COUNTRY>/` comes from there; every other Microsoft app comes from the platform artifact as before.
   Only that folder is read, by ranged requests, and only the entries the run needs (US 28.5: about 55 MB for Base
   Application, System Application and Business Foundation, plus the 65 KB central directory).
2. **Cache.** The country's apps are kept in `artifacts/<build>/country-<country>/apps/`, apart from the platform's
   (same file names, other contents). A platform copy cached by an earlier w1 run is not used for an app the country
   has of its own. With `--no-download` and nothing cached, the run stops with a message; it never falls back to the
   platform's app.
3. **Demo data.** `artifacts/<build>/demo/<country>/BusinessCentral-<COUNTRY>.bak`, downloaded as for w1. With an
   explicit `--country`, the extracted-artifact folder `~/.bcartifacts.cache/sandbox/<build>/<country>/` is looked in,
   and no other country's backup ever answers. The demo company of a backup with several companies is the one named
   `CRONUS International Ltd.`, else the only company whose name starts with `CRONUS` (US: `CRONUS USA, Inc.`);
   otherwise `--test-data-company` is needed. `--no-test-data` never looks for, downloads or reads a backup, in any
   country.
4. **Compile cache.** The country is part of the key of every layer (`country_seed`); w1 keys are unchanged. The
   layers also differ by source hash, so this is a second guard.
5. **Validation.** The code is lower case letters or digits, 2 to 8 characters. Before anything is loaded, the feed's
   `indexes/<country>.json` must exist (404: "the artifact feed has no such country"). The build list for a country
   run is that country's index. An unreachable feed does not stop a run that can be served from the cache.
6. **Not supported with a country**: `--company-data package` (the RapidStart package comes from the platform
   artifact); the run refuses the combination.
7. **Buckets.** Microsoft's `Tests-*.Source.zip` of a country artifact are given to `alrunner test` as paths, like the
   w1 ones; their layout is the same, so nothing else is needed. The runner does not download buckets.

### Measured (28.5.54151.56408, Customizations pair, no test data, empty cache)

Compile cache cold build: w1 24.1 MB in 42.1 s, then US 25.6 MB in 43.3 s; the two share nothing. Of the 8,581 `.al`
files of the US Base Application, 7,732 are identical to W1, 366 differ and 483 exist only in US (none only in W1); System Application and Business Foundation are identical.

