# ADR-018: Defaults chosen in the second round of corpus fixes

- Status: Accepted (defaults chosen without owner review, 2026-10-13/14)
- Date: 2026-10-14

## Context
Branch `fixes-2` made 42 more corpus tests pass. Most of the work follows tests that pin the
behaviour. A few choices go a little beyond what the corpus pins. None changes a seam. Each can be
replaced by a probe result. The assumptions are also in `docs/probes-needed.md`.

## Decision
Decided without owner review (2026-10-13/14):

1. **A failing guarded `Codeunit.Run` inside a page background worker does not undo the caller's
   pending rows.** The corpus pins that a guarded run in a worker is allowed and that the worker's
   commit does not commit the caller's rows. It does not pin the failing case. Options: refuse it, or
   treat it like the commit (the worker's transaction is separate). Chosen: separate, because the worker
   cannot write to a table, so it has no changes of its own to undo.
2. **A direct `Commit` and an unguarded `Codeunit.Run` in a worker stay refused.** Only the guarded run is
   pinned.
3. **`Session.LogMessage` is a no-op in every overload.** Telemetry is not visible to AL code. The corpus only
   needs the call to return.
4. **A modal page whose `OnQueryClosePage` raises an error closes with `None` after the second message.**
   This is pinned. A modal page whose trigger answers `false` and is left open by its handler is still
   refused, because no test pins it.
5. **`SessionSettings.RequestSessionUpdate(false)` passes a copy of the settings to the handler and needs
   the handler to answer true.** `RequestSessionUpdate(true)` writes User Personalization and is refused. A call
   with no handler is refused instead of raising `Unhandled UI: SessionSettings`, to leave the Unhandled UI
   texts alone until they are re-pinned.
6. **An `InitValue` that BC cannot evaluate (`0DT`, a closing-date literal) raises in `Init()` once AL has
   written a field, and on the first read of an untouched record.** Reading it in a record where another
   field was written is refused.
7. **`Session.IsSessionActive(id)` answers false for an id other than the running session's.** The running
   session's own id is refused.

Held back, not merged: the branch `fixes-2-subscriber-lifetime-held` refuses a static subscriber only when a
call changes its global variables, instead of whenever the codeunit has any. It gains 2 tests and loses 2
that had passed only because a sibling test was refused and left no rows behind, so `DeleteAll(false)` on a
table with delete subscribers never ran. It needs the probe for `DeleteAll(false)` with subscribers first.

## Consequences
Each default is a small function that a probe result can replace.

## Conformance
`crates/al-test-cli/tests/suite/fixes2.rs`, `crates/al-vm/tests/hostvalues.rs`, the unit tests in
`crates/al-builtins/src/native.rs`.
