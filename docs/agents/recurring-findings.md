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
  - "opened by the test's OpenEdit" is not `mode == Edit`: a page AL code ran and the test caught with Trap has the
    same mode. Use the one helper `PageState::test_open_edit` (al-vm `pages/state/mod.rs`), not a per-path check;
  - page type, host type, part kind, linked or unlinked part;
  - page properties (Editable, InsertAllowed, AutoSplitKey, DelayedInsert, SourceTableTemporary);
  - triggers declared by extensions (page, table and report extensions) and event subscribers;
  - what the trigger bodies do, not only which triggers exist;
  - the exact test call sequence (what comes before and after the pinned call);
  - a Write between two measured calls checked by its control index only: its validation is page and table code
    (control and field OnValidate, subscribers) that can write Rec, set the key or call CurrPage.Update. Require that
    the write ran no trigger, or that Rec changed only in the typed field;
  - data type, length, sign, position in a filter or expression;
  - record state: temporary, stored, dirty, saved by which call;
  - a flag of a verdict read as a fact about the grant: `Deny { indirect: false }` says the calling object has no object
    grant, not that the session level is blank. Require the session levels the pin measured (al-runner PR 8033).
  - BC version: a gate that one major measured is not a gate for the others;
  - "the test assigned X" read from "the buffer differs from the stored row": a copied or stale buffer also
    differs. Require first that the buffer is the stored row's own image (SystemRowVersion, SystemId, how it
    was read).
  - "the test's filter" read from "filter group 0": page code (OnOpenPage, the record handed to Page.Run) writes group 0
    too. Keep which fields the test filtered, with the typed text, on the page state and require that (al-runner PR 8009).
  - the record handed to Page.Run checked by its filters and key only: its field values are state the page's code reads
    too. A probe that hands a fresh variable measured blank fields: require them, or measure a record after Get or with
    a field assigned (al-runner PR 8009).
  - an Enum fact read from the probe's enum, whose ordinals equal its positions (0, 1, 2): BC shows an Enum filter by
    ordinal. Require ordinals equal to positions and the measured positions, or measure an enum with gaps (al-runner
    PR 8009).
  - a stored origin or state compared with a value built from that same state (`o == f(o)`): the copied
    fields are never checked. Compare with the session's current value (commit count, transaction).
  - an error fact checked by code and kind only: `Error('')` and an error after a skipped save pass too. Require the
    text that is not empty and the step before it (the save ran, nothing else wrote).
  - several entry points that each check a part: one function must prove every fact of the pinned cell, and every
    entry point must call it.
  - a fact kept as session state about a path (how a file was written, whether it is open): list every call that can
    set it and every call that can clear it. The fact must be set only by the measured call and counted for every
    handle, not only for handles that met the shape when they opened.
  - a step rule that reads only the state after the step: the state before the step is a pinned fact too
    (hidden to hidden is not shown to hidden). After a new path ran, the steps the pin makes next are facts
    as well: let only those through (`PinnedEnd`).
  - the arguments of a call whose failure is now caught: a stream or by-reference argument, or a stream an earlier
    call handed over, has effects that the failed call's reply does not carry. Require the measured argument kinds.
  - a body-shape check that accepts any call statement or any condition: Rec and CurrPage methods and procedures of
    the page pass too. Require the kind of callee and the kind of condition the pin has.
  - a body with branches (`case`, `if`) checked by its form only: the pin measured one branch. Require the branch that
    ran (a run-time fact), and the exact branches.
  - a static check over the lowered body (writes, reads, effects): lowering removes exactly the statements it could not
    read. Count a routine that `Engine.blocked` names, or a body with a statement lowered to `Refused`, `Fail`, `TddStub`
    or `FitOrRefuse`, as doing anything (al-runner PR 8015).
  - "Microsoft's app" read from the publisher string alone: the app under test can name "Microsoft" too. Require a
    dependency app as well (al-runner PR 8015).
  - a pinned call checked by its name only: the pin also has its outcome (a `Next` that moved, a `GoToKey` that found its
    row, a `SetValue` that did not raise). Read the calls from the log that keeps outcomes (`tree_calls`, through
    `test_calls_all_succeeded`), not from the names (`test_calls`).
  - facts checked one by one where the pin measured pairs: host type and part kind, or step and host type, are
    measured cells. List the measured cells and require membership, not each factor alone.
  - whether the object exists in the database: TableType = Temporary, a Removed table, Enabled = false, FlowField and
    FlowFilter. A rule about related fields must name which of them count.
  - a value that does not carry its origin (a Blob handed to a Variant or a parameter): a gate on the field kind cannot be
    checked there. Pass the kind from the read site, or do not claim it in the entry (al-runner PR 7951).
  - a name-resolution shortcut checked against variables only: check every kind of declaration that can own the name
    (local, parameter, return value, global, procedure of the object), each measured (al-runner PR 7973), and every
    implicit record that can supply the name: OnRun, a page, a table, a data item, the request page SourceTable of a
    report or xmlport, a `with`. A diagnostic for a name the lookup did not resolve is emitted only in the object kind
    and routine kind alc measured, by equality (al-runner PR 7980).
  - an events exception list copied from a sibling row: the exception is true only where the caller raises the event
    (`format_unsupported_by_layout`). Check who raises each excepted event for the new row's shape (al-runner PR 7979).
  - an error raised for a line before the line is known to be the pinned kind: a parser that reads line by line and
    raises the pinned error from the value, before it checks the key, also raises it for comments, wrapped flow
    collections and other valid input. Check every fact of the pinned line (key, value, position) before the error
    (al-runner PR 8017).
  - a "same value, nothing to do" shortcut: skipping the write also skips the validation. The corpus pins that SetValue of
    the held value still runs OnValidate (testpage/TestPageRuntimeContracts.al:199). Require no control, field or extension
    validate trigger and no subscriber before the shortcut (al-runner PR 8029).
  - `page_declares_within` checks page triggers, properties, extensions and subscribers, not control triggers and not table
    code. A gate whose text says "no trigger" needs the control check (`controls_are_rec_fields`) and the table check
    (`table_code_is`, `table_runs_no_code_for`) too (al-runner PR 8029).
  - a "cannot raise" list of builtins or operators checked by name only: the argument count, the argument values
    (a format text, a Where text, a lock flag), the operand types (Integer and Date addition overflows) and the
    properties of the field named (a CaptionClass runs AL code in FieldCaption) decide whether the call raises or runs
    code. Require each by equality, and run every listed name directly with a bad argument before hand-back
    (al-runner PR 8035).
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
- Before hand-back, read each clause of the entry text against one line of the gate. A fact the text names
  (the empty page, OpenEdit, "the action committed", "assigned from a Variant") and no line checks is a finding:
  add the check, or narrow the text.
- When a native check is replaced (for example by Microsoft's code run as AL), reread every entry that names the old check,
  not only the entry the pull request adds (al-runner PR 7979).
- Do not mark where the assumed change has no reader on that page: a pinned clean pass turns into a flagged
  pass. Check `--top-assumptions` for corpus passes under the new id.
- Regenerate `docs/probes-needed.md` (`--assumptions-doc`) when an entry changes. Do not delete a row while
  unprobed shapes remain.
- The flag must be raised on every pass through the assumed path, at the place the path runs, not at a later
  sync point that a test can skip (close, test end, an undo).
- A mark or flag on a value must survive every copy path: a second stream on the same buffer, `push_str`,
  a copy into another variable. Clear it only when the content is gone.
- A mark set on a shared buffer at a read or a write says nothing about a later move by another holder, and a buffer that is
  empty after Insert, Modify, Clear or Init looks like a new one. Take the fact from the expression at the move site (assignment
  right-hand side, call argument) and from the record's own state, not from the buffer (al-runner PR 7951).
- A mark on a record variable that its own methods follow must also end where another object writes the
  variable's buffer (RecordRef.SetTable, Page.GetRecord). The call that starts the mark is checked as strictly
  as the call that reads it: its arguments, its triggers and subscribers, and the variable's state.
- A mark on a record variable that is ended by a list of calls misses the next platform fill (Page.RunModal handing back the
  picked row, SetSelectionFilter). End it at the one place a record goes by reference into platform code, and exempt by name
  only the calls the pin made (al-runner PR 7951, round 3).
- A buffer written again after Insert, Modify or Init has the flags of one written in memory. A state the probe names (mem,
  ins) is a fact of the record's history: keep it on the record, set by the calls the probe made (al-runner PR 7981).

## 7. State that outlives its shape (6 of 34)

A flag set for the pinned shape must be cleared on every move, close, host change and exit. A value tied to
one run or session must use that run's or session's id, not a shared slot or a per-session counter.

A run-time guard that replaces a static refusal: list every later path that runs the same code (moves, refills, parts), and
check that the flag the guard reads is set somewhere (`grep -rn` the field: a flag with a reader and no writer guards nothing).
The guard covers every kind the static refusal covered (a dialog guard that lets Error through runs what main refused)
(al-runner PR 8034).

## 7a. A value stamped at a measured landing outlives the landing (al-runner PRs 7929 and 7932)

A value put on a record at a measured landing is read after steps the landing does not include. Tie the value to its landing:
every later page step that does not make the page read again (a test call on the host or the part, a page-code call such as
`CurrPage.Update`, a write to the store) must end it, and every other site that writes the same value (a second stamp
function) must go through the owner's allowlist. End the value where it can be READ from, not only on calls made on the page that
holds it: code of another page that calls into it (a host calling a part's procedure), a rollback, and a write or an error inside
a kept call also leave the measured shape. Keep the store state of the landing (writes and rollbacks) with the value and compare
it at each read. Take the store state from the store's own change counter, not only from the places in the VM that count
writes: a call that writes another way (Truncate) is missed. A control whose source is an expression is code that reads Rec: a
value read of it is a read of the stamped value. A value that is on the record only while one call runs is also there for code
that call lets in: a UI handler of the test that calls the page again. End the window at any test call on the page tree that
starts inside it.

Prefer the allowlist form of the same rule (round 3 of PR 7932): keep the value off the record (unknown, a read refuses), put
it on only while the measured read runs and its landing facts still hold, and take it off when that read ends. Then no list of
"events that end the value" exists that the next reader or the next kind of write can be missing from.

## 8. Citations (6 of 34)

- Cite `file:line`, not only a test name.
- The cited line must run the same move with the same kinds of source and target as the cell it is cited for (a field read
  is not a record assignment; a stored record is not a temporary one) (al-runner PR 7981).
- A citation to a probe that is not in the pinned corpus yet is an assumption entry that cites the open
  probe PR, not a plain citation. "Probe queued" is true only when the spec is written down.

## 9. Caps, budgets and ratchets (5 of 34)

Never raise a cap, budget or allowance. Measure best of 3 and state the noise. An allowance only goes down.
A perf cost on a hot path (a check on every write) is measured before and after.
- A new guard in the builtin dispatch (`run_plain_builtin`, `run_builtin`) runs on every call. Gate it with a cheap flag or a
  bit on the call site, as `sqlcount`, `placeholder_output` and `rendered_text` do, and run `perf_vm` on main and on the
  branch before hand-back (al-runner PR 8005).
- A "walk from the newest row" shortcut assumes keys rise. Measure with keys that fall too (al-runner PR 8005).

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

## 13. Microsoft text in fixtures (al-runner PR 7979, and PR 7949 before it)

A stand-in of a Base Application object in a Rust fixture is written from the pinned behavior, never from the object's
source (CLAUDE.md hard rule 7). Only object ids and names, field ids and names, and procedure and event signatures may
match. Before hand-back, compare the fixture's lines with the files in the artifact cache; only signatures, one-word
lines and one-statement assignments of an answer value (`Success := true;`) may match.

## 14. A shape the checker now lets compile, followed only to the checker (al-runner PR 7980)

A checker change that lets a new shape compile: follow the name into lowering. Two declarations the checker tells apart
may share one slot there (a declared global `Rec` and the implicit Rec of a codeunit with TableNo). Run the shape and
read a value that tells them apart.

## 15. A comment says "X members are pinned" while the gate takes the whole type (al-runner PR 8017)

A doc comment or area-table row says "X members are pinned" while the gate takes the whole type (any text, any
Integer). The pins measured a few values. Name the measured values in the comment and gate on them (rounds 1 and 2:
`yaml_text`, `array_item`, `inner_value`).
