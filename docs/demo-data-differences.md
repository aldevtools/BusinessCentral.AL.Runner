# Where the demo backup differs from the database Microsoft's tests run on

The test data of a run is the Cronus backup of the BC artifact (ADR-020). Microsoft's tests were written and gated on a
test database that is set up further. Where a test needs a setup value the backup lacks, the run fails or is refused for
a reason that is not a runtime defect. This file lists the differences that are proven, with the evidence, so that a
failure is not chased in the runtime twice. Each row was found by reading the backup (BC 27.5.46862.48827,
`BusinessCentral-W1.bak`) and confirmed by an experiment: the same tests, run with the value set by an install codeunit
of a scratch app (`al-runner test <test app> <scratch app>`), pass.

| Table, field | In the backup | A test needs | Effect (main 0b414672 or 23cc1e8f, full run) |
|---|---|---|---|
| Sales & Receivables Setup, `Posted Prepmt. Inv. Nos.`, `Posted Prepmt. Cr. Memo Nos.` | empty | a series | "Specify the code for the number series that will be used to assign numbers to posted sales prepayment invoices" and "You cannot get a No. Series Line with empty No. Series Code." in the prepayment codeunits |
| Purchases & Payables Setup, the same two fields | empty | a series | the same, for purchase prepayments |
| Purchases & Payables Setup, `Return Shipment on Credit Memo` | No | Yes (`PurchaseHeader."Return Shipment No. Series"` is only set then) | `PeekNextNo('')`: "You cannot get a No. Series Line with empty No. Series Code." in ERM Purchase Credit Memo and its neighbours |
| Sales & Receivables Setup, `Return Receipt on Credit Memo` | No | Yes | the same, for sales |
| Whse. Worksheet Template, Whse. Worksheet Name | no rows (0 pages) | a template of type Pick | "There is no Whse. Worksheet Template within the filter. Filters: Type: Pick": 55 failures in the full run of main 23cc1e8f (Service Order Warehouse Orange 33, SCM Pick Worksheet 14) |
| Warehouse Journal Template and Batch, Warehouse Employee, Bin Content | no rows (0 pages) | a template of type Item | "There is no Warehouse Journal Template within the filter. Filters: Type: Item": 9 failures (SCM Assembly Item Tracking, SCM Whse.-Asm. To Stock) |
| Item Journal Batch | one row (ITEM/DEFAULT) | a batch for the OUTPUT template | "There is no Item Journal Batch within the filter. Filters: Journal Template Name: OUTPUT" (SCM Order Tracking) |
| Service Cost, Service Zone | no rows (0 pages) | a cost with an account | "There is no Service Cost within the filter. Filters: Account No.: <>'', Service Zone Code: ''": 18 failures (Service - Price Management, Service Line Update Validation) |
| General Posting Setup | 14 rows; the product groups `FREIGHT` and `MISC` exist, the rows (`DOMESTIC`, `FREIGHT`) and (`DOMESTIC`, `MISC`) do not | those rows | "The General Posting Setup does not exist ... Gen. Prod. Posting Group='FREIGHT'" (6.0SP1 - VAT 1 to 1, 35) and `'MISC'` (ERM Invoice Discount And VAT, ERM VAT Serv. Charge, SCM Copy Document Mgt. and others) |
| VAT Posting Setup | 13 rows; (`DOMESTIC`, blank product group) is missing | that row | "The VAT Posting Setup does not exist ... VAT Prod. Posting Group=''" in ERM Prepayment III and Service Warehouse Integration (2 failures; not proven further) |
| Cost Type, Cost Journal Template, Cost Journal Batch, G/L Budget Name | no rows (0 pages) | at least one | "There are no records within the filters specified for table Cost Type", "The Cost Journal Template table is empty", "The G/L Budget Name table is empty": about 55 failures (ERM Cost Acc. Allocations, ERM CA Cost Journal, ERM KPI Web Service) |
| Value Entry | 828 rows, every one with `Cost Posted to G/L` equal to `Cost Amount (Actual)` (none unposted) | unposted entries when SCM Costing Batch's lazy `Initialize` runs `PostInvtCostToGL` (the MessageHandler expects "value entries have been posted to the general ledger") | "Assert.ExpectedMessage failed ... Actual: There is nothing to post to the general ledger" in the first test; `isInitialized` is never set, so every later test of the codeunit repeats `Initialize` and fails at "Unhandled UI: RequestPage 795" or "The following UI handlers were not executed: MessageHandler". 42 of the 68 failures in Tests-SCM-Costing (scm-costing-6 run). Not proven by a scratch app |
| General Ledger Setup, `Additional Reporting Currency` | `EUR` in the BC 28.1 and 29.0 backups (read with `al-bcdb read ... --select "Additional Reporting Currency,LCY Code"`, company CRONUS International Ltd.; blank in the 27.5 backup; LCY is GBP in all) | empty | SCM Inventory Costing II `PostPurchaseInvoiceWithChargeAssignment`: "Cost per Unit must be equal to '0' in Value Entry ... Current value is '1'" (`ItemJnlPostLine.RecalculateCostPerUnit` keeps the rounded cost per unit when the Source Currency Code differs from the ARC), and 10 tests after it in the codeunit. Found by instrumenting Item Jnl.-Post Line |
| Inventory Posting Setup (RESALE and the other groups), Interim and WIP accounts | blank | an account | "WIP Account is missing in Inventory Posting Setup" and "Inventory Account (Interim) is missing" in SCM Manufacturing and Costing (33 + 26 + 11 follow-ons in Tests-SCM-Manufacturing and Tests-SCM-Costing); the failing row is the one `FindLast` picks, so it depends on the order of the rows |
| Responsibility Center (company CRONUS International Ltd.) | no rows (0 pages) | at least one | "The Responsibility Center table is empty" (ERM Sales Quotes `SalesQuoteCreation`) and the tests behind it in the codeunit. Read with `al-bcdb tables`; not confirmed by a scratch app |
| Sales & Receivables Setup, `Direct Debit Mandate Nos.` | empty | a series | "Direct Debit Mandate Nos. must have a value in Sales & Receivables Setup" (ERM SEPA Direct Debit Test `SalesInvoiceDueDateOutRange`) and 21 tests behind it. Read with `al-bcdb read`; not confirmed by a scratch app |
| Service Mgt. Setup, `Service Quote Nos.` | empty | a series | "Service Quote Nos. must have a value in Service Mgt. Setup" (`SearchServiceQuoteByDocNoShowResults` and 4 behind it). Read with `al-bcdb read`; not confirmed by a scratch app |
| Inventory Posting Setup | 13 rows; (EAST, FINISHED) is missing (EAST has RESALE only) | that row | "The Inventory Posting Setup does not exist ... Location Code='EAST',Invt. Posting Group Code='FINISHED'" (ERM Sales Document `LocationforReturnOrder`). Read with `al-bcdb read`; not confirmed by a scratch app |
| FA Depreciation Book, `FA Posting Group` | blank for every fixed asset (FA000010 and the others, book COMPANY) | a posting group | "FA Posting Group must have a value in FA Depreciation Book: FA No.=FA000010, Depreciation Book Code=COMPANY" (ERM Purchase Payables `ArchivePurchaseOrder`). Read with `al-bcdb read`; not confirmed by a scratch app |
| General Posting Setup, (DOMESTIC, MANUFACT) and (DOMESTIC, RAW MAT) | the rows exist; `Sales Line Disc. Account`, `Purch. Line Disc. Account` and `Sales Inv. Disc. Account` are blank (RETAIL, SERVICES, ZERO have 6910 and 7140) | an account | "Sales Line Disc. Account is missing in General Posting Setup ... MANUFACT" and "Purch. Line Disc. Account must have a value" (ERM Invoice Discount And VAT, two tests, and 5 behind them). Read with `al-bcdb read`; not confirmed by a scratch app |
| Item Journal Batch ITEM/DEFAULT, `No. Series` | blank (`al-bcdb read ... --table "Item Journal Batch"`: the one row has blank No. Series and Posting No. Series) | a series, so `Calculate Whse. Adjustment` finds a next document no. | "Enter the document no." (Dialog) in SCM Inventory Item Tracking and the other tests that call `Library - Warehouse.CalculateWhseAdjustment` on that batch: 8 failures. Not confirmed with a scratch app |
| Inventory Posting Setup | 13 rows; `OUT. LOG.` has `RESALE` only (read with `al-bcdb read ... --table "Inventory Posting Setup"`) | `OUT. LOG.` with `FINISHED` | "The Inventory Posting Setup does not exist ... Location Code='OUT. LOG.',Invt. Posting Group Code='FINISHED'": 12 failures in SCM Revaluation-II and others. Not confirmed with a scratch app |
| Inventory Setup, `Internal Movement Nos.` | empty (`Inventory Movement Nos.`, `Inventory Pick Nos.` and `Inventory Put-away Nos.` hold I-MOVE, I-PICK, I-PUT; read in a probe test of "SCM RTAM Item Tracking-II") | a series | "Internal Movement Nos. must have a value in Inventory Setup" in `RegisterInventoryMovementWithLotTrackingWhenBinTypesDoesNotExist`. That test first moves every Bin Type row into a temporary buffer and deletes them, and a Commit inside it (item posting) keeps the deletion when it then fails. The tests after it in the codeunit find no Bin Type `RECEIVE`: "The Bin Type does not exist. Identification fields and values: Code='RECEIVE'" (`PutAwayForWhseReceiptWhenLotConsistsOfSeveralItemEntries`, `PutAwayForPostedWhseReceiptWhenLotConsistsOfSeveralItemEntries`). The backup does hold the rows; the runtime is right to lose them. With the field set in the codeunit's Initialize (experiment, scratch copy of the test app) both tests get past Bin Type and fail at the next step, see the next row |
| Location (the White location of "SCM RTAM Item Tracking-II"), `Use Put-away Worksheet`; Warehouse Setup, `Whse. Internal Put-away Nos.`, `Whse. Internal Pick Nos.`; Whse. Worksheet Template of type Movement | `Use Put-away Worksheet` is Yes when the two tests above post the receipt (read in a probe event subscriber: `OnCodeOnAfterCalcShouldCreatePutAway` has `Use Put-away Worksheet` Yes, so no put-away is made); the two Whse. numbers are empty; no Movement template | the flag back at No; the series; a template | "There is no Warehouse Activity Line within the filter. Filters: Activity Type: Put-away" after the receipt is posted. Tests that run before them set the flag with `UpdateWarehouseAndBinPoliciesOnLocation(LocationWhite, true, ...)` and fail on the missing Whse. Worksheet Template, the missing Whse. Internal numbers or the missing template before their tear-down puts it back; a Commit has kept the change. Inferred from the failing tests listed, not proven by a scratch app that sets all three |

## Proof

The scratch app was one install codeunit that sets the fields above (`Posted Prepmt. Inv. Nos.` to `S-INV+`/`P-INV+`,
`Posted Prepmt. Cr. Memo Nos.` to `S-CR+`/`P-CR+`, both credit memo flags to true, `P-CR+` ending and warning number to
`109999` and `109995`; the last change is no longer needed, see below). Same binary, same tests:

| Codeunit | Without the scratch app | With it |
|---|---|---|
| Tests-Prepayment (all, 567 tests) | 86 passed, 102 failed, 379 not run | 414 passed, 17 failed, 136 not run |
| 138037 O365 Cancel Purch Cr Memo Dim. | 0 passed, 1 failed, 6 not run | 7 passed |
| 137028 Purch. Correct Cr. Memo | 1 passed, 1 failed, 27 not run | 24 passed, 1 failed, 4 not run |
| 134398 ERM Sales/Purch. Correct. Docs | 26 passed, 1 failed, 29 not run | 52 passed, 4 not run |

Most of the not run tests are the codeunit rule that a test after a failed one is not run when it draws random numbers,
so one setup value blocks a whole codeunit.

## What this means for the runtime

Nothing in the runtime is wrong here: the AL code of the Base Application runs as written. Making these tests run is a
choice about the database, not a fix: the runner would have to prepare the demo company the way Microsoft's test
database is prepared. That changes the data a test sees, so it is an owner decision (see docs/owner-review.md); the scratch app
is a one-file way to try it.

## The `P-CR+` range was a runtime defect, not data

No. Series Line `P-CR+` starts at 109001 and ends at 1010999. `NoIsWithinValidRange` takes Code[20] parameters and checks
`CurrentNo > EndingNo`. The runtime ordered two Code values as text, so `'109001' > '1010999'` was true and the series
read as used up ("no unused posted credit memo numbers").

The corpus pins Code apart from Text: text/text-methods/TextCompare.Codeunit.al `CodeComparison` (lines 205-246)
shows `'10' < '9'` is No for Code (observation `tenlt`). The fix orders two Codes of plain digits (no leading zero) as
numbers in the `<`, `<=`, `>`, `>=` operators. Leading zeros, signs and mixed codes are not pinned and stay as before.
It is registered as the assumption `code-digits-compare-as-numbers`. Test: `crates/al-vm/tests/language/code_compare.rs`.
Results without any scratch app are in the hand-back; the prepayment and return-flag rows above remain data findings.

## Tables the backup holds no rows of (found with the empty-table text of FindFirst, FindLast and FindSet)

`al-bcdb read <bak> --table <T> --company CRONUS` returns no rows for these, and the tests below call `FindFirst` or `FindSet`
without creating a row first, so they fail with "The <table> table is empty." (DB:TableIsEmpty, pinned by
record/filters/P68C_EmptyFinds.al). The backup holds 5 customers and no Service Item.

| Table | Tests that fail on it (Tests-Misc, Tests-SCM, Tests-ERM) |
|---|---|
| Service Item | Test Planning/Req.Wksh (PlanForServiceDemandNetChange, PlanAfterNeededByDateNetChange, MultipleDemandsSameDayNetChange) |
| Warehouse Entry | SCM Warehouse Unit Tests, 27 tests |
| Cost Journal Template | ERM CA Cost Journal, 16 tests |
| Phys. Invt. Counting Period | SCM Physical Inventory tests that call FindFirst on it: "The Phys. Invt. Counting Period table is empty." (14 failures that were hidden behind refusals until the grouped-query and link-filter assumptions let those codeunits run) |
| Cost Type, Responsibility Center | ERM Cost Accounting - Tables and Pages, ERM Sales Quotes (SalesQuoteCreation) |

Not confirmed by a scratch app that seeds the rows; the other rows above are.

Cost Accounting - Pages, date filter: TestCostBudgetPerCostCenterMatrixValidateFiltersForAmountTypeNetChange and 3 siblings read
`MatrixForm.FILTER.GetFilter("Date Filter")`. The matrix part sets that filter on its Rec in `SetRecordFilters`, which only runs for a row
(OnAfterGetRecord). The backup holds no Cost Type rows, so no row is read and the filter stays empty. Not confirmed with seeded rows.
## Group 1 of ms-fails-2 was runtime, and mostly fixed by earlier merges

"Location Code GU... cannot be found in Location" (95), "Bin Code ... Bin" (about 190), "Item Tracking Code SN-PRON" (24) and
"Warehouse handling is required" (about 107) were values the test had just created. They are gone from the failures after
the merges that followed the full run of main 0b414672 (3273 failures then, 924 on main 23cc1e8f). The
remaining part is a runtime defect fixed on branch ms-fails-2: a `where()` condition of a TableRelation that names the
validated field itself (`Item Variant.Code where("Item No." = field("Item No."), Code = field("Variant Code"), ...)` of Prod.
Order Line, Assembly Header and Assembly Line) read the old, blank value of the field, so it contradicted the key and the
variant "cannot be found". It reads the value being validated now (pinned by record/table-relation/RelationWhereValidated.al,
test `crates/al-vm/tests/store/relation_where_enum_filter.rs`).

How the data rows above were proven: `bcdb tables BusinessCentral-W1.bak` lists the page count of every table of the
company; the tables above have 0 pages, and a scratch test that counts the rows through the runtime agrees.

## Package company against backup company (`--company-data package`, BC 27.5, branch package-failures-1)

The package company is built from `NAV27.5.W1.ENU.EXTENDED.rapidstart` (the DemoTool's Legacy data, as Microsoft's Legacy
lane uses it). Per test, against the backup company, on Tests-SMB, Tests-Misc and Tests-ERM.

### Reader defects found and fixed (class b)

Together these were most of the extra failures of the first package run (SMB 327 passed against 542 for the backup).

1. **Hyphen in a field name.** The package writes `-` as `_` (`Unit_AmountRoundingPrecision`, `Bill_toName`,
   `Asm_to_OrderShptBinCode`) and drops every other punctuation character. The reader dropped the hyphen too, so every
   field with a hyphen in its name (all Sell-to, Bill-to, Ship-to, Pay-to, Buy-from, Unit-Amount, Over-Receipt, ... columns)
   was left out of the company. Symptom: `The value of ROUND parameter 2 is outside of the permitted range` from
   `Unit-Amount Rounding Precision` = 0, and `Validation error for Field: No.`. `al-rapidstart` `element_name`.
2. **`<DimensionsasColumns>` is a table flag.** Three tables (Customer among them) carry it beside `TableID`. It was read
   as a record with no fields, which became a row with a blank key (a Customer with `No.` = ''). Symptom: `The record in
   table Customer already exists. ... No.=''` on every `Customer.Insert()` that takes its number from the series.
3. **AutoIncrement fields the package does not hold.** `Dimension Value."Dimension Value ID"` is not in the package. The
   database numbers rows it inserts, so the rows get 1, 2, 3 in package order (registry entry
   `package-auto-increment-from-one`, flagged passes; no probe can measure it, it rests on the import tool).
   Symptom: `Dimension Value ID must have a value in Dimension Set Entry` (Sales-Calc. Discount Test, ERM Sales Quotes).

### Real differences of the package (class a)

Microsoft disables most tests that need these on the Legacy lane (`src/DisabledTests`, shown as `(DIS)` in the comparison).

- **Trial Balance Setup** is not in the package. Company-Initialize inserts an empty row; the backup's row names a row and a
  column definition. O365 Trial Balance: `Row Definition must have a value in Trial Balance Setup` (4 tests, all disabled by Microsoft).
- **No ledger entries or documents.** The package holds no G/L, customer, vendor or item ledger entries and no sales or
  purchase documents. Tests that count them fail: Normal DemoData and RS Pack Content - Evaluation (CountSalesDocuments,
  PostSalesInvoices, CountContacts, ...), Analysis View Table-UT, Test Suppress Commit in Post, ERM Source Currency. All
  disabled by Microsoft except Normal DemoData.CountContacts, which passes in the backup company only.
- **Country/Region** lacks codes the tests ask for (`The Country/Region does not exist`): 35 ERM Financial Reports III and PEPPOL tests, disabled by Microsoft.
- **Currency Exchange Rate** rows for some currencies are missing or differ (ERM Exch. Rate Adjmt. Apply and others, disabled).
- **G/L Account Category** is not in the package, but the package's G/L accounts name subcategory numbers. The install
  code (`InitializeAccountCategories`) therefore resets all 287 to 0, and `GL Acct. Categories Page Tests.TestCannotDeleteUsedGLAccountCategory`
  finds no account with a category. Not disabled by Microsoft; it fails in the package company.
- **The first G/L account with Direct Posting and no subcategory** is a purchase-type account, so
  `O365 Activites Tests.CalcAverageCollectionDays` posts a payment with `Bal. Gen. Posting Type` Purchase
  (`Account Type Customer and Bal. Gen. Posting Type Purchase is not allowed`). Data difference of the account list.
- **Item Journal Line** holds one row with a blank key in the package. A test that inserts a blank-key line meets a
  duplicate key (`ItemTemplateChangeServiceTypeItemJnlLine`); the runtime refuses the deferred duplicate-key insert (class c, below).

### Runtime gaps the backup hides (class c, not fixed: unpinned)

- `TextBuilder.Append` of a Variant that holds a Date, Decimal or DateTime (Assert.RecordCount builds its error text with
  `RecRef.FieldIndex(i).Value()`). Refused as `text argument 0 of type an undefined Date`. The assertion only fails when the
  record count differs, so the backup company never reaches it. 20 tests of RS Pack Content - Standard/Evaluation are not run because of the refusal.
- Duplicate-key Insert into a table with an insert trigger (Vendor, Item Journal Line) is refused ("whether it runs is not pinned").

### Result (package company, BC 27.5)

Passed / failed / not run. The backup company is the reference.

| Bucket | Package before | Package after | Backup |
|---|---|---|---|
| Tests-SMB | 327 / 48 / 655 | 542 / 19 / 469 | 542 / 15 / 473 |
| Tests-Misc | 1403 / 181 / 1661 | 1620 / 82 / 1543 | 1603 / 108 / 1534 |
| Tests-ERM | 3259 / 543 / 5531 | 5922 / 121 / 3290 | 5846 / 85 / 3402 |

(SMB before ran with a 3 s test time limit; the others with the default 20 s.)
