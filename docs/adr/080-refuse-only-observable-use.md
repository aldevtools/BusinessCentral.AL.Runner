# ADR-080: Refuse an unmeasured value at its observable use, not at its read

- Status: Accepted (owner, 2026-11-24)
- Date: 2026-11-24

## Context

Owner rule, 2026-11-24: "If something can not affect a test it should not be refused."

Several rules refuse when code *reads* state whose BC value the run does not know, or when a test codeunit's static reach
*could* read it. They refuse even when the value is then discarded, overwritten before use, or only sent to telemetry.
Examples:

- A SingleInstance codeunit that partner login subscribers changed in an unmeasured shape: every read of any of its
  globals refuses (`al-testrun/src/partner_login.rs:292-298` marks the instance, `al-vm/src/interp.rs:1137-1141` and
  `al-vm/src/single_unmeasured.rs:45-54` refuse the read). On main this stops the whole NP Retail suite at the test
  runner's own row write (`al-testrun/src/runtime/suite_rows.rs:75`, :94; issue 8945).
- A failure after an unfinished test, and a codeunit after a carry chain member: refused on static reach, with no read at
  all (`al-testrun/src/runtime/written.rs:196-211`; `carry_after.rs:41`, `corpus.rs:2430-2438`).

The full list, with costs, is the audit (local file, `~/.cache/al-runtime/adr-observable-use/audit.md`).

Two mechanisms already refuse at the use instead of the read:

- `Value::SqlCount` (al-builtins `sqlcount.rs:1-37`; al-vm `sqlcount.rs:1284-1325` `guard`): a marked value that may be
  stored, passed on, held as a List or Dictionary value and given to `Session.LogMessage`; every other use refuses.
- `Value::Unknown` (al-builtins `value.rs:171-176`, `ops.rs:51-57`; al-vm `lib.rs:2639-2641`): an opaque value with only a
  reason. It sits in a record buffer or a by-value parameter; every other use refuses with its reason.

## Decision

### 1. The rule

A value whose BC value the run does not know (an **unmeasured value**) is refused only at its first **observable use**.
A read alone is not a use. A test in which no unmeasured value reaches an observable use runs and is judged as usual.

### 2. Observable uses

ADR-009 lists what is observable. An unmeasured value is used observably when it can decide any of these:

1. **Control flow**: a condition (`if`, `case`, `while`, `repeat`, `exit` of a Boolean test), a comparison, a `case`
   label. What runs next, and in what order, is observable.
2. **Any operation that can raise**: arithmetic, conversion to a narrower type, `Evaluate`, text functions, a Dictionary
   key, a filter. Whether an error occurs is observable.
3. **Values a test can see**: a return value or `var` result that code then uses as above, `Error`, `Message`,
   `Confirm`, `StrMenu`, `GetLastErrorText`, an assert.
4. **Database state**: a record field written by `Insert`, `Modify`, `Rename`, `Validate`, `TransferFields`, or used in a
   filter or key.
5. **Pages and the host**: a page or TestPage field, a request page, a file, a stream, an HTTP request, a .NET call.

The allowed uses form an **allowlist**. Everything not on it counts as observable:

- reading it into a local or global of the same type (no conversion that can raise);
- passing it by value to a parameter of the same type, and returning it from a procedure;
- holding it as a List or Dictionary *value* (never a key), as `SqlCount` does;
- overwriting it, and dropping it (scope end, `Clear`);
- `Session.LogMessage` arguments. This clarifies ADR-009: telemetry the runtime would emit is not observable. The runtime
  emits none. The allowance is for the builtin only. AL subscribers on telemetry events are AL code and follow this
  rule. Pin for "no error": SqlCounters.al `P6924_Counters_FormattedInTelemetryDimension_NoError`.

### 3. How the runtime proves a value cannot reach an observable use

By construction, not by flow analysis:

- A read of marked scalar state returns an **opaque value**: the reason it is unmeasured and the declared type, no
  content (as `Value::Unknown`). It holds no number or text, so nothing can be computed from it.
- The allowlist of section 2 passes it on unchanged. Every other operation on it refuses with its reason.
- So a run that ends without that refusal did not depend on the value. "Discarded" and "overwritten before use" need no
  analysis: the value is simply never used.
- A full write of a known value to a marked global clears the mark of that global. Marks become per global. A global the
  runtime did not change stays marked, because BC's run may have changed it.

Static reach (`written.rs` `ReachFacts`) stays as a pre-filter only: a codeunit that cannot reach the state runs with no
marks at all.

### 4. What stays refused at the read (tracing gaps)

Refusal is still right where the value cannot be carried opaquely (hard rule 4). Each of these is a **tracing gap**,
refused at the read as today, with its own runner issue:

- reference-typed state: Record and temporary-record globals, List, Dictionary, Codeunit, JsonObject and other Json
  types, DotNet, streams, RecordRef and FieldRef;
- a marked global passed as a `var` argument;
- a marked value converted to Variant and then to another type;
- table rows (class T in the audit): rows carry no mark, so code that reads or writes a table with unmeasured rows
  stays refused at the touch;
- number sequences and isolated storage keys whose state after a rollback is not measured (`failed_test.rs`,
  `storage_checkpoint.rs`, `isolated_storage_reads.rs`): what is unknown is whether the sequence or key *exists*, so the
  call itself can raise on BC. The call is an observable use and stays refused at the call.

A tracing gap uses the typed refusal of ADR-071 with area `tracing-gap`, so `--top-refusals` counts them apart.

### 5. Assumption registry and flagged passes

- An unmeasured value that reaches no observable use needs no assumption. The pass is **clean**: the outcome cannot
  depend on a value with no content.
- Whether a read gives an opaque value or an assumed one is decided when the state is marked, not at the use. Marked
  refused: the read gives an opaque value, and its observable use refuses (no assumption can make an opaque value run).
  Marked assumed: an assumption entry gives BC's value for that shape, the read keeps the content and marks the entry
  (a flagged pass, hard rule 3).
- Assumed values (`SingleMark::Assumed`, `single_unmeasured.rs:49-52`) keep their content, because the entry gives it.
  In this ADR they still mark the entry at the read. Moving that mark to the first observable use needs a marked value
  *with* content. That is a later amendment.
- The rule never turns a refusal into a pass without either the proof of section 3 or an assumption entry.

### 6. A branch on an unmeasured value (owner decision, 2026-11-24)

Section 2 counts every branch as observable. That is stricter than "can not affect a test". The owner's own example
shows the cost. Azure AD User Subscribers (System Application, `AzureADUserSubscribers.Codeunit.al:26-64`) is the only
known reader whose values go only to telemetry, and it is not recovered by this ADR as written:

- `GetInformation` starts with `if Initialized then exit` and later branches on `IsAzure` (:31, :36, :60): control flow.
- It adds `IsAdmin` as a dimension through `Language.ToDefaultLanguage`, which formats the Boolean (:64): `Format` is
  not on the allowlist.

Two choices:

- **A. Keep the strict rule** (as written). Simple and provable. Recovers only values that are dropped, overwritten,
  stored or passed on. 9034 and the NP lazy-init readers stay refused.
- **B. Track the branch.** Inside a branch on an unmeasured value, every write marks its target, every observable use
  refuses, and the arm not taken must have, by static reach, no observable use and only writes that get marked. Add
  `Format` of an unmeasured value to a marked text, as `SqlCount` does (pinned: SqlCounters.al
  `P6924_Counters_FormattedInTelemetryDimension_NoError`). Recovers 9034 and other telemetry-only readers. Costs a
  static check of the arm not taken, and a mark on control regions in the VM.

Owner decision, 2026-11-24: **A now, B next.** The ADR builds A first (steps 0 to 3). B follows right after as a committed
step, sized by the counting diagnostic of step 0. It is not optional.

### 7. Build order

0. **Counting diagnostic** (no behavior change, off by default): for each marked read, record the first use by kind
   (drop, overwrite, store, pass, LogMessage, control flow, raise-capable op, output). Run on NP Retail and the MS
   buckets. This sizes the gain before any rule changes.
1. **Static reach to read** (class S): rules that refuse because a codeunit *could* read the state mark the state
   instead, and refuse at the read. Today's largest rule-shaped gap (ADR-064: 1581 tests in the BC 29 MS run; NP: 320
   codeunits refused with no read).
2. **Read to use** (class R): scalar SingleInstance globals, `CurrentTransactionType`, the SystemRowVersion value, as
   section 3. NumberSequence and IsolatedStorage stay refused at the call (section 4).
3. Per-global marks with write clearing (section 3, last point).

Each step is its own pull request with neighbor apps for each allowed and each refused use.

## Consequences

- Fewer refusals where the value is discarded, overwritten or logged. The audit says the NP gain is small: the NP
  readers branch on the value (lazy-init flags), and NP's real cause is false changes (`al-vm/src/state_stamp.rs:97-100`,
  a temporary-record global always counts as changed). Step 0 measures the real gain first.
- An opaque value reuses `Value::Unknown`, or an additive variant next to it (ADR-010, no seam change). It is checked
  only on the cold path of a marked read, so the speed budget (ADR-031) pays one branch per global read, which exists
  today (`interp.rs:1139`).
- The refusal moves later in the run. A refused test may have run more code first. ADR-064 already rolls back and bounds
  what a refused test leaves.
- The refusal text names the use, not the read. Reviews check the allowlist, not each read site.

## Conformance

- Rust tests in al-vm: per allowlisted use, a marked value passes unchanged; per observable use kind, the refusal.
- Neighbor apps under `tests/neighbors/observable-use/`: a login memo read and dropped, overwritten, logged (run clean);
  branched on, compared, written to a field, shown in `Message` (refused).
- No corpus case can pin "not observable"; the corpus pins only the behaviors the allowlist runs (telemetry: SqlCounters.al
  P6924).
