---
title: "Test data"
weight: 60
---

## The demo company

Microsoft's tests expect the CRONUS demo company, and many of them fail on missing
setup data without it. A run that uses Microsoft's apps therefore restores the
CRONUS company of the same build by default. A table is read the first time a
test touches it. The rest is never read.

AL Runner looks for the backup in this order:

1. `~/.bcartifacts.cache/sandbox/<build>/w1`
2. The artifact cache.
3. Microsoft's artifact feed, once (about 520 MB, progress on stderr).
4. A backup of another build of the same Business Central version. The output says which.

If none can be had, test data is turned off and the run says why. The summary
lists what was loaded and every exclusion: tables the runtime could not rebuild,
tables the reader failed on (with its text), and columns left out.

| Flag | Effect |
|---|---|
| `--no-test-data` | Run with an empty database. |
| `--test-data [PATH]` | Use a specific `.bak` or `.bacpac`, or insist on test data. A missing backup then stops the run. |
| `--test-data-company NAME` | Which company to restore. Exact name or prefix. |
| `al-runner warm [PATH.bak]` | Open the backup once and keep its page map and catalog in the cache. The first open verifies the whole file (2 to 3 seconds on a 900 MB backup); later opens take milliseconds. Run it before a demo on a new machine. |

The reader behind this is `al-bcdb`, a Rust reader for `.bak` and `.bacpac` files
that needs no SQL Server.

## Other countries

`--country CODE` selects the country version (`us`, `gb`, `de`, and the other names
of the artifact feed). The default is `w1`. Only this flag selects it. `app.json`
and AL-Go settings are not read. The country's own Microsoft apps replace the w1
ones, and the demo data is that country's backup (about 600 MB the first time).
It cannot be combined with `--company-data package`.

## The RapidStart package company

```bash
al-runner test Tests-ERM.Source.zip --company-data package
```

`--company-data package` builds the company the way Microsoft's Legacy test lane
does: a fresh company, `Company-Initialize`, then the RapidStart package of the
artifact. `--package PATH` names a different `.rapidstart` file. A company built
from the package needs `--server-url` for `Company-Initialize`.

## Microsoft's demo company

Microsoft's CI builds the company of some test lanes with its own tools (the
DemoTool for the Legacy lane, the Contoso Demo Tool for the IntegrationTest and
Uncategorized lanes). AL Runner does not build these companies by default. The
support for it is experimental and will be redesigned.

## Known gaps in the demo data

Microsoft's tests were written for a test database that is set up further than the
CRONUS backup in the artifact. When a test needs setup the backup lacks, it fails.
This is a gap in the data, not a defect of AL Runner. Most rows below were confirmed
by setting the value in a small extra app, after which the tests passed. The others
were found by reading the backup.

| Table and field | In the backup | Typical message |
|---|---|---|
| Sales & Receivables Setup and Purchases & Payables Setup: `Posted Prepmt. Inv. Nos.`, `Posted Prepmt. Cr. Memo Nos.` | empty | "You cannot get a No. Series Line with empty No. Series Code." |
| Sales & Receivables Setup: `Return Receipt on Credit Memo`; Purchases & Payables Setup: `Return Shipment on Credit Memo` | No | "You cannot get a No. Series Line with empty No. Series Code." |
| Sales & Receivables Setup: `Direct Debit Mandate Nos.` | empty | "Direct Debit Mandate Nos. must have a value in Sales & Receivables Setup" |
| Service Mgt. Setup: `Service Quote Nos.` | empty | "Service Quote Nos. must have a value in Service Mgt. Setup" |
| Inventory Setup: `Internal Movement Nos.` | empty | "Internal Movement Nos. must have a value in Inventory Setup" |
| Whse. Worksheet Template | no rows | "There is no Whse. Worksheet Template within the filter." |
| Warehouse Journal Template and Batch | no rows | "There is no Warehouse Journal Template within the filter." |
| Item Journal Batch | only ITEM/DEFAULT, with no No. Series | "There is no Item Journal Batch within the filter." or "Enter the document no." |
| Service Cost, Service Zone, Service Item | no rows | "There is no Service Cost within the filter." |
| General Posting Setup | (DOMESTIC, FREIGHT) and (DOMESTIC, MISC) missing; some discount accounts blank | "The General Posting Setup does not exist" or "... is missing in General Posting Setup" |
| VAT Posting Setup | (DOMESTIC, blank) missing | "The VAT Posting Setup does not exist" |
| Inventory Posting Setup | some location and group pairs missing; Interim and WIP accounts blank | "The Inventory Posting Setup does not exist" or "WIP Account is missing in Inventory Posting Setup" |
| Cost Type, Cost Journal Template, Cost Journal Batch, G/L Budget Name | no rows | "The Cost Journal Template table is empty" |
| Responsibility Center, Warehouse Entry, Phys. Invt. Counting Period | no rows | "The ... table is empty." |
| FA Depreciation Book: `FA Posting Group` | blank | "FA Posting Group must have a value in FA Depreciation Book" |
| General Ledger Setup: `Additional Reporting Currency` | EUR in the 28 and 29 backups, blank in 27.5 | cost amounts that differ by rounding |

These were found on the W1 backup of Business Central 27.5, unless the row says
otherwise.

## Microsoft's test apps

To run Microsoft's own tests, with the right company for each test codeunit and
Microsoft's disabled tests skipped, see
[Running Microsoft's tests](microsoft-tests.md).
