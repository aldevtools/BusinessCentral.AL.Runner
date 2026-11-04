# ADR-047: The corpus database's demo rows in corpus runs

- Status: Rejected (owner, 2026-10-21)
- Date: 2026-10-21

## Context

The corpus runs on real BC in a database that holds demo rows the runtime's corpus database does not have. Corpus bumps 37
and 38 pin some of them:

- **Company Information** (table 79), one row: Primary Key `''`, Name = CompanyName(), Name 2 `''`, Address `5 The Ring`,
  Address 2 `Westminster`, City `London`, Post Code `W2 8HG`, County `''`, Country/Region Code `GB`, Phone No.
  `0666-666-6666`, VAT Registration No. `777777777`, Registration No. `''`, Picture HasValue Yes (its bytes are not pinned)
  (session-system/company/CompanyInformationValues.al:QG_CompanyInformation_CorpusRow). The report dataset header
  tests read more of it: Fax No. `0666-666-6660`, Bank Branch No. `BG99999` and the logo (misc 70535 PW_Dump_Lines*,
  report 78603 Header_AddressLines_AsInCorpusDatabase, report 67755).
- **Accounting Period** (table 50), 36 rows from 2025-01-01 to 2027-12-01 with pinned flags
  (CompanyInformationValues.al:QG_AccountingPeriod_CorpusRows).
- **General Ledger Setup** (table 98): LCY Code `GBP`, Global Dimension 1 Code `DEPARTMENT`, Global Dimension 2 Code
  `CUSTOMERGROUP`, Additional Reporting Currency `EUR` from BC 28 and `''` on BC 27
  (record/captions/QdCaptionClassAreas.al:808 QF_GLSetup_CorpusRow; gate `ProbedValuesDifferFrom28`).

Every other field of these rows is unknown. These cases fail or are refused for the missing rows (per major): 67858 x2,
78603, 70535 x3, 67755, 84363, 86504 Literals_FieldCaption, Expressions_FieldCaption and QF_GLSetup_CorpusRow (the
CaptionClass areas 1 and 101 refusal in `pages/rows.rs` `refuse_setup_caption`).

Two things the runtime does not have today:

1. A corpus run's database holds what the install baseline writes (ADR-013 decision 5, ADR-046) and nothing else. A row
   that no AL code wrote, seeded for corpus runs only and not for `alrunner test` (whose data comes from CRONUS, ADR-020),
   is new.
2. A row whose fields are partly unknown. ADR-019 decision 7 refuses unknown columns of platform tables statically, at
   lowering, for every routine that names such a column. That rule does not carry over: Base Application code names many
   fields of General Ledger Setup and Company Information, so a static refusal would refuse a large part of Base
   Application for every corpus test that reaches it, also when the field is never read on the path the test runs.
   A Picture whose HasValue is Yes but whose bytes are unknown has no form in the store at all.

## Decision (proposed)

1. **Seed the pinned rows in corpus runs only.** `oracle/oracle.toml` (or a file next to it, generated from the corpus
   tests that pin the values, as `pinned_columns.rs` is) lists the rows per BC major. The corpus job writes them into the
   install baseline after the install code ran, in the session company, and commits them. `alrunner test` never seeds
   them. Where the install code already wrote a row with the same key (General Ledger Setup), the pinned fields replace
   its values and the row's other fields become unknown.
2. **Unknown fields refuse at run time, where their value is used.** A stored row carries an optional mask of unknown
   fields (an additive field of the store row and of a record's state). A read of a masked field (a field access, a
   FieldRef value, Format of the record, TransferFields, a filter or sort on it that decides which rows come back, a
   Modify that would write it back unchanged) raises `Signal::Unsupported` naming the table and field. Writing a value
   into the field clears its mask bit. Blob and Media fields can be "has a value, bytes unknown": HasValue answers, a
   read of the bytes refuses.
3. Until this is accepted, the cases above stay failing or refused as they are now.

## Options considered

- **Static refusal as in ADR-019.** Simple and refuses before the test starts, but it refuses every routine that names an
  unknown field, which in Base Application is most of the posting and setup code. Rejected for the Base Application
  tables; it could still serve a small table whose fields only test code names.
- **Seed with plausible values.** Rejected: hard rule 3 (no invented demo data, ADR-009).
- **Use the CRONUS backup (ADR-020) for corpus runs.** Its values differ from the corpus database (a different demo
  company), so it would make other corpus reads wrong. Rejected.

## Consequences

- A new kind of row state in the store and in records (the unknown-field mask), checked on every field read of a record
  whose row has one. Rows without a mask pay one branch.
- The corpus job and `alrunner test` start from different databases on purpose; the corpus database is documented in
  one place.
- About 11 corpus cases per major can run once it is built.

## Conformance

- Unit tests in `al-data` for the mask (read, write clears, filter refuses).
- The corpus cases named above.

## Outcome

Rejected by the owner on 2026-10-21. The runtime does not seed the corpus database's demo rows. The corpus tests that read
demo rows are changed to write their own setup rows. The tests that pin the demo rows themselves stay refused.
