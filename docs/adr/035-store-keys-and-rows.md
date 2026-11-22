# ADR-035: Store keys and rows

- Status: Accepted (2026-10-15, decided by the orchestrator on the owner's delegation; see docs/owner-review.md)
- Date: 2026-10-15
- Related: ADR-003 (AL-shaped store trait), ADR-004 (snapshots), ADR-005 (company in every table identity), ADR-009,
  ADR-032 (company ids), review `docs/reviews/2026-10-compile-ir-review.md` (R7)

## Context

A record operation on a temporary table costs about 28 000 instructions (review benchmark `RecordLoop`). The
profile: `key_cmp` and `ops::compare` 18 %, `Vec<Value>` clones 9 %, `malloc` and `free` 6 %, the `im` B-tree lookup
3 %. Per operation the VM also:

- matches the method name as a string (`al-vm/src/records.rs:894-939`, fixed by ADR-032);
- scans all fields of the table for an unevaluable `InitValue` (`records.rs:940`);
- clones company and user `String`s into an `AccessContext` (`al-vm/src/companies.rs:75-95`);
- deep-clones the `TableDef` when an ad-hoc key is set (`records.rs:55-59`).

`MemStore` keys tables by `(String, u32)` and rows by `Key(Vec<Value>)` compared value by value with the collation
(`al-data/src/memory.rs`). Rows are `Vec<Value>` and are cloned on every read.

## Decision

1. **Encoded keys.** A primary or secondary key is encoded once into a byte string whose `memcmp` order equals
   `key_cmp` (collation folded in for Code and Text, sign-flipped big-endian numbers, a terminator per part).
   `im::OrdMap<KeyBytes, Row>`.
2. **Shared rows.** `Row = Rc<[Value]>` in the store (`Arc` where test data is shared between threads); a read
   shares the row, and the record buffer copies on its first write.
   Built (issue 8570): the record buffer is `al_builtins::record_fields::Fields`, shared until its first write.
3. **Company by id** (`CompanyId` from ADR-032); `AccessContext` holds ids.
4. **Table facts precomputed** in `TableDef`: has an unevaluable `InitValue`, temporary, CRM, the key list including
   an ad-hoc key slot (no clone).
5. The `Store` trait (ADR-003) does not change. Order of rows, filters and every typed outcome stay as they are.

## What changes at which seam

- `al-data`: `MemStore` internals; `Row` type (a type alias today; if callers depend on `Vec`, they change once).
- `al-vm`: `records.rs`, `companies.rs`, `testdata.rs` read rows through the new type.
- The `Store` trait and its contract suite: unchanged.

## Proof of zero behaviour change

- A property test: for random values of every key type, `memcmp(encode(a), encode(b)) == key_cmp(a, b)`, including
  Code with mixed case and digits, Text with non-ASCII, negative Decimals, empty values, Guids, Dates with the closing
  flag, Options and Enums.
- The `al-data` contract suite unchanged and green.
- Corpus (every major) and Tests-ERM, Tests-SCM with test data: outcomes identical (`tools/compare_junit.py`).
- `tools/baseline_rule_check.sh`: 0 differences.

## Migration (each step merges green)

1. Precomputed table facts and `AccessContext` by id (small, VM only).
2. Shared rows.
3. Encoded keys behind the property test.

## Consequences

- Expected gain: 2 to 3 times on record-heavy code (estimate from the profile shares).
- Blocked by: ADR-032 step 6 (company ids) for decision 3 only. Independent of ADR-033 and ADR-034.
- Cost: 5 to 8 agent-days. Risk: sort order is observable (ADR-009); the property test is the guard.

## Conformance

The key-order property test; the `al-data` contract suite; ADR-031's `RecordLoop` budget.
