# ADR-017: Defaults for error collection, partial records and media

- Status: Accepted (defaults chosen without owner review, 2026-10-13/14)
- Date: 2026-10-14

## Context
The second builtins round (branch `builtins-2`) needed a few choices the corpus does not settle. Each is
additive: no seam changes. They are listed so the owner can review them. The assumptions are also in
`docs/probes-needed.md`.

## Decision
Decided without owner review (2026-10-13/14):

1. **Several collected errors.** Closing an outermost `[ErrorBehavior(Collect)]` scope that holds more than one
   error raises `Multiple errors occurred. <first message>`. The corpus pins only that the text names a
   "Multiple errors occurred..." summary and contains the first message. Options: refuse the case (loses one
   test), or this text. Chosen: this text, because the pinned test passes and no pinned behaviour is contradicted.
2. **`HasCollectedErrors()` outside a scope answers true.** The corpus documents it (finding D in
   errors/errorinfo/TestErrorInfoType.al) as true by arithmetic in the platform, and the tier failed an assertion of false.
3. **A collectible `Error(ErrorInfo)` inside a scope returns and the raiser runs on.** The corpus raisers have
   nothing after the call, so both readings pass. Chosen: continue, which matches the documented wording.
4. **Media and MediaSet fields hold a Guid; the objects live in a session store.** This stands in for the
   platform tables "Tenant Media" and "Tenant Media Set" until ADR-013 gives them rows. Options: wait for
   ADR-013. Chosen: the stand-in, so 9 corpus tests pass now and 16 more pass once `Temp Blob` is compiled.
5. **`AddLoadFields` and `SetBaseLoadFields` without a prior `SetLoadFields` answer false and do nothing.** The
   corpus comments say so; no test asserts the result.
6. **The call depth ceiling is 1000.** The corpus pins that 990 calls complete and 5000 are refused. Past the
   ceiling the run is refused (Unsupported), because the platform's text is not pinned.
7. **`LockTimeout` defaults to true; `LastUsedRowVersion` never reports 0.**

## Consequences
Each default can be replaced by a probe result without changing a seam.

## Conformance
`crates/al-vm/tests/errorcollect.rs`, `loadfields.rs`, `media.rs`, `exec.rs` (`a_990_deep_chain_completes`),
`builtins.rs`.
