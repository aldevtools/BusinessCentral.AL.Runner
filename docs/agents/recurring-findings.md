# Recurring review findings

The findings review raised again and again (34 review hand-backs of runtime pull requests, up to
2026-11-12). The count is the number of hand-backs that had the finding. The implementer checks its own diff
against this list before it hands back. The reviewer checks this list first, then looks for the rest.

A new finding that shows up in two or more pull requests gets a row here, in the same change that fixes it.

## 1. A new rule runs on shapes no case pins (29 of 34)

The gate of a new path is wider than the cells the corpus or the probe measured. Each round of review then
finds the next neighbor shape. Write the gate in allowlist form:

- One predicate next to the owner of the behavior. It requires every pinned fact **positively, by equality**.
  Never "the new path minus listed exceptions". Never a subset check ("has at least these properties").
- Each fact in the predicate cites its pin (file:line).
- Facts that were missed most often:
  - open mode: OpenEdit, OpenView and OpenNew are three exact flags (`mode != New` lets OpenView in);
  - page type, host type, part kind, linked or unlinked part;
  - page properties (Editable, InsertAllowed, AutoSplitKey, DelayedInsert, SourceTableTemporary);
  - triggers declared by extensions (page, table and report extensions) and event subscribers;
  - what the trigger bodies do, not only which triggers exist;
  - the exact test call sequence (what comes before and after the pinned call);
  - data type, length, sign, position in a filter or expression;
  - record state: temporary, stored, dirty, saved by which call;
  - BC version: a gate that one major measured is not a gate for the others;
  - "the test assigned X" read from "the buffer differs from the stored row": a copied or stale buffer also
    differs. Require first that the buffer is the stored row's own image (SystemRowVersion, SystemId, how it
    was read).
  - a stored origin or state compared with a value built from that same state (`o == f(o)`): the copied
    fields are never checked. Compare with the session's current value (commit count, transaction).
  - an error fact checked by code and kind only: `Error('')` and an error after a skipped save pass too. Require the
    text that is not empty and the step before it (the save ran, nothing else wrote).
  - several entry points that each check a part: one function must prove every fact of the pinned cell, and every
    entry point must call it.
  - a fact kept as session state about a path (how a file was written, whether it is open): list every call that can
    set it and every call that can clear it. The fact must be set only by the measured call and counted for every
    handle, not only for handles that met the shape when they opened.
  - the arguments of a call whose failure is now caught: a stream or by-reference argument, or a stream an earlier
    call handed over, has effects that the failed call's reply does not carry. Require the measured argument kinds.
  - whether the object exists in the database: TableType = Temporary, a Removed table, Enabled = false, FlowField and
    FlowFilter. A rule about related fields must name which of them count.
- Probe the neighbors yourself: flip one fact at a time and check that the path refuses or keeps main's
  result.

## 2. A shape main answered now refuses or answers differently (13 of 34)

A neighbor shape outside the new rule must keep main's result: main's answer, or main's refusal. A new
refusal on a shape main ran is a regression, often in Base App code that partner tests reach.

- Add Rust tests that out-of-scope shapes keep main's outcome. Compare with a main binary.
- Keep a refusal only as wide as its reason. Check its order: look the fact up first, refuse only when it
  matters.

## 3. Text that does not match the code or CI (comments 13, pull request body or ADR 9)

- Code comments that overstate what is pinned ("all eight", "never stored"), name a moved function, or sit
  on the wrong item after a splice.
- PR body claims that the code does not do ("outside shapes keep the old path", "stays refused"), a refused
  cells list that misses cells, file names that changed, "green on 27.5 and 28.5" when that CI failed.
- Before hand-back, read the PR body and the area table against the final diff and the actual CI result.
- Run the tests of every crate whose tests call the changed path, not only the crates whose source changed:
  `grep -rn` the changed member or refusal text under `crates/*/tests` first.

## 4. More than one owner (10 of 34)

- A rule placed in a shared helper with many callers (a truth test, every list, every listing) changes
  callers nobody measured. Put it at the one site the pin covers.
- A second copy of a check or a text beside its owner. Call the owner.
- Matching by the wrong key: error text instead of the raise site, source text instead of the IR, a short
  name with first match instead of the resolver, trigger names instead of trigger kinds.
- State in a session slot instead of the run or page it belongs to.

## 5. New refusals without a Rust test (10 of 34)

Every new refusal gets a Rust test that reaches it, unless a corpus case reaches it.

## 6. Assumption registry and probe rows (registry or row 7, flag lost 5)

- A path without a pin runs only through an assumption entry (CLAUDE.md rule 3). The entry text matches what
  the code does (for example "no subscriber" when the code checks none).
- Regenerate `docs/probes-needed.md` (`--assumptions-doc`) when an entry changes. Do not delete a row while
  unprobed shapes remain.
- The flag must be raised on every pass through the assumed path, at the place the path runs, not at a later
  sync point that a test can skip (close, test end, an undo).
- A mark or flag on a value must survive every copy path: a second stream on the same buffer, `push_str`,
  a copy into another variable. Clear it only when the content is gone.
- A mark on a record variable that its own methods follow must also end where another object writes the
  variable's buffer (RecordRef.SetTable, Page.GetRecord). The call that starts the mark is checked as strictly
  as the call that reads it: its arguments, its triggers and subscribers, and the variable's state.

## 7. State that outlives its shape (6 of 34)

A flag set for the pinned shape must be cleared on every move, close, host change and exit. A value tied to
one run or session must use that run's or session's id, not a shared slot or a per-session counter.

## 8. Citations (6 of 34)

- Cite `file:line`, not only a test name.
- A citation to a probe that is not in the pinned corpus yet is an assumption entry that cites the open
  probe PR, not a plain citation. "Probe queued" is true only when the spec is written down.

## 9. Caps, budgets and ratchets (5 of 34)

Never raise a cap, budget or allowance. Measure best of 3 and state the noise. An allowance only goes down.
A perf cost on a hot path (a check on every write) is measured before and after.

## 10. Checks run on one major only, or on an old binary (4 of 34)

Run `--baseline-diff` on BC 27, 28 and 29. Rerun Microsoft suite comparisons with the head binary, not the
first commit. A rule gated to BC 28 must still answer the BC 27 cells the corpus measures.

## 11. The answer says more than the pin (4 of 34)

- An exact length or byte count where only the sign (`> 0`, `<> 0`, `= 0`) is pinned.
- A return value that depends on the content, used where only the statement form is pinned.
- Bytes beyond the prefix that the corpus pins.
- An error text composed from parts, or a shown text that is not in the corpus.

## 12. Unknown reach counted as safe (3 of 34)

A check that proves "nothing else can see this" fails when its reach analysis has unknown entries (RecordRef
with a run-time id, dynamic object ids), unless the owner named that kind of unknown as safe.
