# ADR-010: Value model

- Status: Accepted (project owner, 2026-10-13); decision 1 amended by ADR-072 (`Value` lives in `al_value`; page state leaves the value crate)
- Date: 2026-10-13

## Context
ADR-008 puts the value model in `al-builtins`. The VM, the builtins and the
store all exchange values, so its shape is a seam. Several value semantics
are observable (ADR-009) and must come from the oracle, not from guesses:
Code normalisation, Text length limits, Decimal rounding, Date closing dates,
record assignment versus `Copy`, codeunit variable sharing.

## Decision
1. **One `Value` enum** in `al_builtins::value`, one variant per AL runtime
   type: `Integer(i32)`, `BigInteger(i64)`, `Decimal(rust_decimal::Decimal)`,
   `Boolean`, `Char(u16)`, `Byte(u8)`, `Text`, `Code`, `Date`, `Time`,
   `DateTime`, `Duration(i64 ms)`, `Guid([u8; 16])`, `Option(i32)`,
   `Enum { id, ordinal }`, `Record(RecordHandle)`, `RecordRef`, `FieldRef`,
   `List`, `Dictionary`, the Json/Xml/stream/builder types, `Codeunit(Instance)`,
   `Variant(Box<Value>)`, and `Null` only as the pre-initialisation state.
2. **Value types copy, reference types share.** Primitives, Text and Code are
   values; `:=` copies them. List, Dictionary, Json*, Xml*, TextBuilder,
   streams and codeunit instances are `Rc<RefCell<..>>` handles; `:=` shares
   the handle. Whether a given AL type is value- or reference-like is taken
   from corpus tests; where the corpus is silent, a probe decides first.
3. **Text keeps UTF-16 semantics** for length, indexing and `MaxStrLen`
   checks, stored as a Rust `String` plus a cached UTF-16 length.
4. **A record is a handle** `{ table, company, field buffer, filters,
   current key, marks, temporary store }`. `:=` copies the field buffer only;
   `Copy` also copies filters and key; `Copy(x, true)` shares a temporary
   table. These rules come from the handoff and are confirmed against the
   corpus `record/` tests before they are relied on.
5. **Defaults** for every type (`0`, `''`, `0D`, empty Guid, first enum value
   etc.) are defined in one place, `Value::default_for(&Ty)`, and covered by
   tests derived from the corpus.
6. **Conversions** (implicit and `Evaluate`/`Format`) live in `al-builtins`
   next to the value model, one function per (from, to) pair, each with the
   corpus test that pins it.

## Consequences
- `al-vm`, `al-data` and `al-host` depend on `al-builtins` for values;
  `al-data` stores field values as `Value` (no second value type).
- Observable conversion behaviour is concentrated in one module, which is
  where Format/Evaluate/rounding risk (handoff "Risks") is reviewed.

## Conformance
Unit tests per variant for copy-versus-share; a corpus-derived defaults table;
conversion functions each cite a corpus test.
