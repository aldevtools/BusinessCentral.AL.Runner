# ADR-060: License-backed permissions: License Permission and the license level of a permission check come from a BC license file

- Status: Proposed
- Date: 2026-10-30
- Related: ADR-006 (no OS access in core crates), ADR-007 (corpus is the oracle), ADR-009 (match BC where observable),
  ADR-013 (Microsoft's apps from their real source; nothing from Microsoft is committed), ADR-019 (platform table
  providers), ADR-027 (scope; open decision 3, permission enforcement), ADR-028 (`serve`)

## Context

### What the runtime does today

The platform table License Permission (2000000043) is answered row by row from corpus pins
(`al-vm/src/platform/host_tables.rs`, `license_permission_get`):

- `platform/lt-platform-rows-fresh/PlatformRowsLicensePermission.al:65-73` and
  `platform/lt-platform-tables/PtCloudTables.al:60-76`: the first rows (TableData 0 all blank, TableData 1..7 all Yes),
  and Table 99999 all Yes.
- `platform/lt-ag-contoso/AgLicensePermission.al`: Codeunit 1, 80, 410, 99999999 and the app's own numbers, one object
  of every type at a low number, the customer range (50000-99999) and the ISV range (70000000+), Codeunit 2000000000
  all Yes, Codeunit 0 all blank.
- `lt-ss1030/SsLicensePermissionObjects.al` (corpus staging branch): seven object types at 50123, 99999, 70000456,
  100000, 1999999999, 2000000000, 2000000001 and 2147483647.

Every other Report, Codeunit or Page from 1 to 99999999 is answered by the assumption
`license-permission-any-object` (all Yes). Any other number, type or wider read is refused. Each new question
(another range, a negative number, a Count) needs another probe.

Permissions are not enforced (ADR-027 owner decision 2, open decision 3). `--ignore-permission-mock` runs tests that
lower their own permissions with every access granted (assumption `permissions-not-enforced`).

### The owner's direction (2026-10-29, 2026-10-30)

Load the license instead of answering row by row:

1. By default, the license that ships with the BC artifacts (the Cronus demo license, `app/Cronus.bclicense` in the
   artifact).
2. A command line flag and a serve option for the user's own license: an on-premises developer license, or a
   specific end customer's license.

The license states which object numbers are licensed, per object type, with which permissions. That answers License
Permission for every number at once, and later gives the license level of each permission check.

Licenses and permissions are tightly coupled in BC. A license can allow an operation only indirectly (for example
Insert on TableData G/L Entry). A direct insert then fails with a **permission** error, unless the calling object
declares the matching indirect permission in its `Permissions` property. A license gap shows up as a permission
error, not a license error.

**Goal (owner, 2026-10-30).** A partner can test, end to end, whether their permission configuration works with a
specific end customer's license: their permission sets, the `Permissions` properties of their objects, and
entitlements where relevant. The partner passes the customer's license file with the license flag and runs their
tests.

### What the license file is (local investigation, 2026-10-30)

Investigated: `Cronus.bclicense` from the BC 27 and BC 29 artifacts and from the BC 28 service tier of the Linux
container. All three files are byte-identical. No `.flf` file was found on this machine; the format of `.flf` is not
covered here.

- **Encoding.** UTF-16 LE text with a byte order mark, CRLF line ends.
- **Layout.** A plain-text header (licensee, product line, version, expiry, a list of granted modules), followed by
  one line of XML.
- **XML.** A `License` root with three children:
  - `Properties`: named, typed values (country code, expiry, licensed object ranges for some features, counts).
  - `PermissionCollections`: one list per object type. Each list holds ranges, each with a first number, a last
    number and a permission bit mask.
  - An XML digital signature (enveloped, RSA-SHA1) with the signer's certificate. The file is signed; it is not
    encrypted.
- **Object types in the file:** Codeunit, FieldNo, MenuSuite, Page, Query, Report, System, TableData,
  TableDescription, XMLPort, and a separate list for limited-usage tables. There is no list named `Table`.
- **Permission mask.** In the object type lists, one bit per permission: almost every range has all five bits set
  (Read, Insert, Modify, Delete, Execute Yes); one System range has only the Execute bit. The limited-usage list uses
  other values, whose meaning is not known. The Cronus license grants nothing indirectly, so the encoding of
  **Indirect** cannot be read from it.

**Mapping to the License Permission columns (to be confirmed by probes):**

| License Permission | License file |
|---|---|
| Object Type | the list's object type; `Table` from `TableDescription` (inferred: Table rows are pinned Yes where only TableDescription ranges exist) |
| Object Number | every number inside a range of that list gets a row with the range's mask; a number outside every range gets a row with every permission blank (pinned: Codeunit 0, 1999999999, 2147483647) |
| Read/Insert/Modify/Delete/Execute Permission | one bit each of the mask (Yes), the Indirect bits not yet known |
| Limited Usage Permission | the limited-usage list (mapping not known, see the disagreements) |

### The license compared with the corpus pins

**Agree.** Every pinned row from 1 to 99999999 (all types) is all Yes; the license grants all five permissions there
for every type. 0, 1999999999 and 2147483647 are outside every range and pinned all blank. 2000000000 and 2000000001
are inside the license's system object range with all five permissions and pinned all Yes for most types. `Database.CheckLicenseFile` of a System
key outside every System range (-1, 2147483647, -2147483647) raises the permission error
`Sorry, the current permissions prevented the action. (System N N Execute: <app name>)`, code `DB:ExecuteObjDenied`;
9999, inside a System range, passes (`al-language-onprem/lt-system-database-onprem/PCheckLicenseFile.al:36-39`).

**Disagree.** These pins are not explained by the license file alone:

1. **Page 2000000000 and 2000000001 on BC 28 and 29: Execute only** (`SsLicensePermissionObjects.al`, `#if BC28PLUS`).
   The license grants all five for those pages, and the file is the same for 27, 28 and 29. BC itself must narrow system
   pages from BC 28 on.
2. **Table 2000000001: Read, Insert, Modify, Execute, no Delete.** The license grants all five for
   TableDescription in the system object range. BC narrows this system table itself.
3. **Limited Usage blank on Table and TableData 18** (`AgLicensePermission.al`). The license lists table 18 in the
   limited-usage list. Either BC does not show that list in this column for this license (an on-premises
   product line), or the mapping is different.

So the license file is the base, and BC's own rules for system objects (2000000000 and up) apply on top of it, per BC
major. Those rules stay pinned by the corpus.

## Decision

### 1. The license file is the reference; the corpus pins check the reader

License Permission rows and the license level of a permission check come from a parsed license. The corpus pins stay
and test the reader: every pinned row must match what the reader produces from the default license. A pin that does
not match is either a reader bug or a BC rule outside the file; the second kind is kept as an explicit override with
its corpus citation (the three disagreements above are the first such overrides).

### 2. Which license is used

- **Default:** the license in the BC artifact of the selected BC version (`app/*.bclicense`), found next to the
  artifact the runner already uses for that version.
- **`--license FILE`** (serve: `license`): any `.bclicense` file the user has: a developer license, a partner license,
  or an end customer's license. It replaces the default for the whole run.
- **No license available** (offline without the artifact, an artifact without a license): the runtime answers exactly
  what it answers today, the pinned rows only, and refuses the rest. `license-permission-any-object` is not used
  then either (see 5).
- A file that cannot be parsed stops the run before any test starts, with the reason. It is never silently replaced
  by the default.
- **The run report names the license used:** its source (artifact default or `--license`), the licensee and the
  expiry date from its header, and the file's hash. The JUnit and JSON reports carry the same values. This tells the
  reader which license a pass or fail belongs to.

### 3. Where the parsed license lives

- A pure parser in a core crate turns the file's bytes into a `License` value: per object type, a sorted list of
  ranges with five permission levels each (Yes, Indirect, blank), the limited-usage list, and the header values the
  report needs. The parser does no OS access (ADR-006).
- The host (the CLI and serve) finds and reads the file and passes the bytes in, as it does with test data and
  dependency apps. The `License` is a host input to the session, held in the engine next to the compiled program,
  shared read-only by all sessions of a run.
- The signature is not verified. The runner is a test tool and the user supplies the file; the run report says the
  signature was not checked.
- **The license is the user's input, read at run time from the user's artifacts or the path the user gives. It is
  never shipped or committed, and never stored in a cache shared across users.** The compile cache key includes the
  license hash only where a cached result depends on the license; the license itself is not written into the cache.

### 4. Which reads it answers

- **License Permission (2000000043):** every read: `Get`, a filtered `FindSet`, `Count`, the whole table in key order.
  Rows are produced from the ranges on demand (no table of 2^31 rows per type is built), then the pinned overrides for
  system objects are applied.
- **`Database.CheckLicenseFile(Key)`:** a key inside a System range passes; outside, the pinned permission error
  (`PCheckLicenseFile.al:36-39`).
- **License Information (2000000040):** the plain-text header lines. Its row layout is not pinned yet; it stays refused
  until a probe pins it, then comes from the header.

### 5. What replaces `license-permission-any-object`

The assumption is retired once the reader serves License Permission. A number is no longer assumed: it is inside a
range of the loaded license or it is not. Rows the license gives for numbers no pin covers are answered from the file
without a flag, because the file is the reference (decision 1). If no license is loaded, the runtime refuses those
numbers instead of assuming them.

### 6. The license in a permission check (design now, enforce later)

Permission enforcement stays a later phase (ADR-027 open decision 3, a separate ADR). This ADR fixes the license side
so that phase can use it directly.

For one operation (Read, Insert, Modify, Delete on TableData; Execute on other object types) BC decides from three
inputs:

1. **License level** L: Yes, Indirect or blank, from the license (this ADR).
2. **User level** U: Yes, Indirect or blank, the best level any of the user's permission sets give (including
   entitlements and inherent permissions where relevant).
3. **Object grant** O: whether the object whose code runs declares this permission in its `Permissions` property.

The model the enforcement phase is expected to build, to be confirmed by probes:

- Direct access is allowed when L = Yes and U = Yes.
- Indirect access is allowed when L and U are each Yes or Indirect and O holds the permission.
- Otherwise BC raises a permission error.

In short: allowed = license ∩ (permission sets ∪ the object's own indirect permissions), where an Indirect level on
either side needs O.

What the corpus pins today:

- Permission sets with Indirect, plus a `Permissions` property: allowed; without the property: the error text
  `Sorry, the current permissions prevented the action. (TableData N <name> IndirectInsert: <app name>)`, code
  `DB:ClientInsertDenied` (`permissions/lt-permissions-mock-indirect/TestIndirect.al:80-81, 102-103`). The corpus also
  pins Read, Modify and Delete (`DB:ClientReadDenied`, `DB:ClientModifyDenied`, `DB:ClientDeleteDenied`) and
  `IndirectRead` and `IndirectModify` texts.
- A license gap on a System object: `DB:ExecuteObjDenied` (`PCheckLicenseFile.al:37-39`).

Not pinned: a denial caused by the **license** on a TableData, Page, Report or Codeunit. The corpus CI runs with the
Cronus license, which licenses everything directly, so it cannot produce one.

**Reporting a denial.** A permission or license failure names the object, the operation and the missing level, using
BC's own error text (ADR-009). Beside it, the runner reports which input denied: the license, the permission sets, or
the missing `Permissions` property. BC's text does not say which; this extra line is a diagnostic, not part of the
error text.

**Relation to `--ignore-permission-mock` and the runner's permission sets.** While enforcement is off, nothing
changes for them: every access is granted and `permissions-not-enforced` and `runner-permission-sets-grant-all` stay
flagged. When enforcement comes, the license level applies under every test, also under `--ignore-permission-mock`
(that flag ignores the sets a test assigns, not the license). A loaded license with enforcement off is still useful:
License Permission reads and `CheckLicenseFile` answer from it.

### 7. Out of scope

- Enforcing permissions (ADR-027 open decision 3; its own ADR).
- License checks that are not permissions: user counts, modules, expiry, entitlement plans of a SaaS tenant.
- Verifying the license signature.
- The `.flf` format of older versions.

## Consequences

- License Permission answers every read, for every number, from one file. The assumption
  `license-permission-any-object` and the per-number probes for ranges go away.
- `--license FILE` lets a partner run their tests against a customer's license. Until enforcement lands this covers
  License Permission and `CheckLicenseFile` only; the partner use case is complete only with the enforcement phase.
- The parser depends on a file format Microsoft does not document. A format change in a later BC major is caught by
  the corpus pins (decision 1) and by a parser error that stops the run.
- BC's own rules for system objects stay as pinned overrides, per BC major. A new major can add more.
- The runner never ships a license. A run without an artifact license answers less than a run with one, and the
  report says which license (or none) was used.

## Open questions (with recommended answers)

1. **Indirect bits.** Which bits of the mask mean Indirect? *Recommended:* read them from a license that grants
   something indirectly (a customer or partner license the owner has locally, inspected locally, never committed), and
   pin the observable result (License Permission shows `Indirect`) in a corpus probe once the corpus CI can import such
   a license. Until then, a mask with unknown bits stops the run with a clear reason.
2. **Limited Usage column.** How the limited-usage list maps to `Limited Usage Permission`, given table 18 is listed
   but pinned blank. *Recommended:* answer the column blank for the Cronus license (pinned), and refuse the column for a
   license whose limited-usage list differs from Cronus, until a probe with a second license pins it.
3. **System object overrides.** Are the three pinned narrowings (Page 2000000000/2000000001 on 28+, Table 2000000001)
   the only ones? *Recommended:* add a corpus probe that reads License Permission for every system object number
   (2000000000 and up) of each type, per BC major, and serve those rows from the pins.
4. **Table vs TableDescription.** Confirm that `Table` rows come from `TableDescription`. *Recommended:* one probe row
   for Table at a number where the TableDescription and TableData ranges differ (needs a license where they differ;
   with Cronus they are equal, so accept the inference and note it in `docs/probes-needed.md`).
5. **Default when the artifact license is missing.** *Recommended:* the pinned rows only, as today, and say so in the
   report. Never fall back to a built-in license.
6. **License-caused denial texts.** Which error text and code BC raises when the license (not the sets) denies a
   TableData operation directly or indirectly, a Page open, a Report run, a Codeunit run. *Recommended:* probes queued
   in `docs/probes-needed.md`; they need a corpus CI run with a restricted license. Until pinned, enforcement of a
   license denial is refused, not guessed.
7. **Signature check.** *Recommended:* no check. The user owns the input, and BC's own behavior with a tampered file is
   not observable to AL code.

## Conformance

- A reader test per pinned row: the reader, given the default license from the artifact, produces every row the
  corpus pins (with the pinned overrides), for BC 27, 28 and 29. These tests run only where the artifact is present
  and are skipped with a reason otherwise; no license file is ever added to the repository as a fixture.
- A parser unit test on a synthetic file built in the test (made-up ranges, unsigned) that covers ranges, gaps, every
  object type list and a mask with unknown bits.
- The corpus pass-rate report: the License Permission and `CheckLicenseFile` cases stay passing.
