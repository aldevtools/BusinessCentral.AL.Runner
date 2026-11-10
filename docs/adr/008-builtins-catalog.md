# ADR-008: Builtins catalog — signatures from the docs, implementations by id

- Status: Accepted (project owner, 2026-10-13); decision 4 amended by ADR-072 (the value model and the catalog move to a stable `al-value` crate; natives and `Ctx` stay in `al-builtins`)
- Date: 2026-10-13

## Context
The type checker and the VM must agree on every builtin: its overloads,
parameter types, `var` parameters, optional and variadic parameters, and
whether its return value may be ignored. Hard rule 4 requires an unimplemented
builtin to fail at compile time, so the checker must also know which overloads
have an implementation.

The corpus repository ships `scripts/al-surface.json`, scraped from
Microsoft's `methods-auto` reference: 91 types, ~1740 overloads, each with a
`syntax` line such as
`[Ok := ]  Record.SetFilter(Field: Any, String: Text [, Value: Any,...])`.
The structured `parameters` arrays in that file drop optional parameters (they
end up inside descriptions); the `syntax` line does not.

Builtin implementations operate on runtime values and, for things like
`Record.Insert(true)` that run triggers, must call back into the VM.

## Decision
1. **Signatures are generated, not hand-written.** `al-builtins` parses the
   `syntax` line of every overload in `corpus/scripts/al-surface.json` into a
   `Signature { receiver, name, params: [{ name, ty, by_var, optional, variadic }],
   ret, ret_optional, instance }` and writes a checked-in, one-line-per-overload
   file `crates/al-builtins/catalog/signatures.txt`. A test regenerates it and
   fails if it is stale, so a corpus bump shows signature changes as a diff.
2. **Corrections are explicit.** Where the scraped docs are wrong or
   incomplete, `catalog/overrides.txt` replaces individual lines, each with a
   comment citing the probe or doc page that justifies it. Never edit the
   generated file.
3. **Each overload has a stable id** (`Record.SetFilter#2`, numbered in
   catalog order per method). Implementations are registered by id in a
   table in `al-builtins`. The checker resolves a call to one overload; if
   that id has no implementation, it emits `AR1001 unsupported builtin` and
   the test codeunit does not compile.
4. **The value model lives in `al-builtins`**, because builtins are the code
   that operates on it. `al-vm` depends on `al-builtins`, not the reverse.
   Native functions receive a `&mut dyn Ctx` (defined in `al-builtins`,
   implemented by `al-vm`) for anything that needs the interpreter, the
   store or the host: running triggers, raising errors, reading the clock.
5. **Global functions** (`Message`, `Format`, `CopyStr`, `Commit`, …) are the
   non-instance overloads of the receiver types AL exposes without a
   qualifier; which receivers those are is decided by corpus usage and
   recorded in the catalog, not guessed.

## Consequences
- Catalog coverage (implemented ids / ids the corpus calls) is a daily metric.
- A doc-scrape error surfaces as a type-check failure on corpus code that BC
  compiles, which is then fixed by an override with a reason.

## Conformance
`al-builtins` tests: the generated file is up to date; every override names a
source; every registered implementation id exists in the catalog.
