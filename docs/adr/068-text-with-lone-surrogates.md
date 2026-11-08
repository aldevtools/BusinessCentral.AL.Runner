# ADR-068: Text that holds half of a surrogate pair

- Status: Proposed
- Date: 2026-11-08

## Context

AL `Text` and `Code` are .NET strings on BC: sequences of UTF-16 units. Any unit can stand alone, also half of a surrogate
pair (U+D800 to U+DFFF). This runtime stores a text as a Rust `String` (UTF-8) inside `Txt` (`al-builtins/src/text.rs`).
A `String` cannot hold a lone surrogate. `Value::Char` is already a `u16`, so a Char can.

### What the corpus pins

Real BC keeps a lone surrogate in a text, and AL code sees it:

| Behavior | Pin |
|---|---|
| `T[2] := 55296` stores the unit; `StrLen` is 3 | `types/lt-text-char-index/TextCharIndexAssignEdges.al:88-116` |
| `CopyStr` and `DelStr` cut a pair in half; `T[i]` reads the half; `StrPos` finds a half | `text/lt-cr-text-leads/CrTextLeads.al:414-439`, `text/lt-text-instance/Index1.Codeunit.al:350` |
| Two one-unit texts (high, low) joined give one valid pair; `foreach` yields 4 units for `'a' + High + Low + 'b'` | `text/lt-foreach-text/ForeachText.al:127-147` |
| `OutStream.WriteText` writes a lone surrogate as U+FFFD in UTF-8 and UTF-16 | `streams/StTEncodings.al:249-265` |
| `XmlDocument.WriteTo` of a text with a lone surrogate raises an `Argument` error | `xml/XmlDocReadWrite.al:813-835` |
| JSON, BigText and stream writes per encoding | `json/W7JsonStreams.al`, `errors/lt-ms-refusals-129/P129StreamWriteText.al`, `errors/lt-ms-refusals-132/P132OutStreamUnicode.al`, `streams/lt-stream-non-ascii/StreamWrittenText.al`, codeunit 80150 |

### What this blocks

From the BC 29 JUnit of 2026-11-08 and `metrics/<major>/not-passing.txt`: **26 cases per BC major** (27, 28, 29 alike).

| Refusal | Cases |
|---|---|
| `assigning Char N to a character of a text (only a UTF-16 character is pinned)` (`al-vm/src/interp.rs`, `write_char`) | 23 |
| `CopyStr that ends up with half of a surrogate pair (not pinned)` (`native/strings.rs`) | 1 |
| A later test of the same codeunit, untrusted after one of the above | 2 |
| `a UTF-16 read of resource ... whose bytes hold an unpaired surrogate` (`moduleinfo.rs`; needs the BC error, not the representation) | 1, not in the 26 |

By area: streams 9, json 4, text 4, types 2, errors 2, misc 1, xml 1, codeunit 80150 (BigText) 4 with its cascade.

Not caused by the representation, so out of scope: the 34 `InStream.Read into a Text without a zero terminator or on a
UTF-16 stream` cases. They need a probe of what `Read` returns, not a new text type.

### Silent U+FFFD today (hard rule 4)

These sites turn half a pair into U+FFFD with no refusal, so a test that hits them can pass or fail on a guessed value:

- `textfns.rs` through `from_u16` (`String::from_utf16_lossy`): `Substring`, `Remove`, `Split`, `ConvertStr`, `PadStr`,
  `DelStr`, `InsStr`, TextBuilder `ToText`, `SetLength`, `Replace` (range), `Insert`, `Remove`, BigText `AddText`, `GetSubText`.
- `json.rs` unescape of `"\uD800"` (`from_utf16_lossy`).
- `platform_native.rs` Base64 decode with `TextEncoding::UTF16`.

No corpus case reaches them with a lone surrogate today. They need a refusal now, before this ADR is built (a separate small fix).

### Size of the code

Measured with `grep -rn` in `crates/*/src` on main of 2026-11-08:

- `Value::Text(` and `Value::Code(`: 1,194 lines in 185 files (al-vm 93+, al-builtins 57+). About 687 of them bind the text
  to read it; the rest build one.
- `text_of(`: 95. `encode_utf16`: 40. `from_utf16`: 22. `char::from_u32` on a Char: 22.
- `Txt` derefs to `&str`. Every reader of a text uses that. No reader has to be changed unless it must see a lone surrogate.

## Options

### a. WTF-8 inside `Txt`

WTF-8 is UTF-8 that also encodes lone surrogates (3 bytes each).

- **Effort:** high. A WTF-8 buffer is not valid UTF-8, so `Deref<Target = str>` is unsound for it. Either every reader
  (about 690 bindings and 95 `text_of`) moves to a new API, or `Txt` keeps a second, lossy `String` for readers, which is
  option c with a harder main buffer. Rust's own WTF-8 type is private to `std`; a crate would be added (license check).
  Joining two halves needs the WTF-8 merge rule at every append.
- **Speed:** the same as today for valid text, plus a check at every append.
- **Risk:** medium to high. Every library call (`format!`, collation, JSON, XML, serde, the .NET wire) still needs `&str`, so
  the lossy copy and the audit of readers remain. It gains nothing over option c.

### b. `Vec<u16>` (or a UTF-16 rope) as the text

- **Effort:** highest. All 1,194 sites in 185 files, every text builtin (textfns, strings, format, evaluate, collation, filter,
  JSON, XML, streams: about 14,000 lines in `al-builtins`), IR constants, constant folding in `al-sema`, the store codec and
  key order (ADR-035), the compile cache format (ADR-022) and the .NET wire change. Every Rust library call needs a
  conversion each way.
- **Speed:** ASCII text takes twice the memory. `Format`, `Evaluate`, JSON, XML and filters convert on every call. The ASCII
  fast paths in `Txt` (byte index for `CopyStr`, `T[i]`) are lost, although `T[i]` on non-ASCII text gets O(1). The VM
  budget (ADR-031) and the corpus instruction budget would likely fail and need new caps.
- **Risk:** high. A rewrite of the text layer for 26 cases, with a large regression surface.

### c. Hybrid: UTF-8 by default, exact units only when a lone surrogate exists

`Txt` keeps its `String`. A text with a lone surrogate also carries a side list of the lone units:

- The `String` holds U+FFFD at each lone position. U+FFFD is one UTF-16 unit, as the surrogate is, so `StrLen` and every
  position (`CopyStr`, `StrPos`, `T[i]` bounds) stay correct without a change.
- `lone: Option<Box<[(u32, u16)]>>`: unit index and real unit, sorted. `None` for every valid text, so valid text pays
  nothing but one pointer-sized field.
- Every constructor normalizes: when a high and a low half become neighbors (`T[3] := 56832` after `T[2] := 55357`, or
  `High + Low`), they merge into the real character and leave the list. Most corpus cases build an emoji unit by unit and
  end in the plain form.

- **Effort:** low to medium. `text.rs` (340 lines), the 22 Char-to-text sites, `write_char`, the join in `ops.rs`, `CopyStr`,
  the 14 `textfns` functions above, and only those readers that the corpus pins for a lone surrogate (list in the decision).
  Estimate: 120 to 160 changed sites, against 1,194 for option b.
- **Speed:** a branch on `lone.is_some()` in `Deref` and `as_str`. `Txt` is hot, so measure with `perf_vm` and
  `perf_budget`; expected within noise, since the field sits next to the string pointer.
- **Risk:** low, if unaudited readers are caught (decision item 3). Without that check, a reader would see U+FFFD silently.

## Decision

Option c.

1. **Representation.** `Txt` gets the side list above. New API: `Txt::from_units(&[u16])` (exact),
   `Txt::from_unit(u16)`, `units() -> Cow<[u16]>` (exact), `has_lone()`, and `audited_str()` (the lossy `String` for a
   reader that is known to match BC). `from_utf16_lossy` is no longer used to build a `Value::Text` or `Value::Code`.
2. **Sources of lone surrogates.** Built in: `Text[i] := Char`, Char to Text (assignment, argument, concatenation), the join
   of texts, `CopyStr`, `DelStr`, `Substring`, `Remove`, `Split`, `PadStr`, `InsStr`, `ConvertStr`, TextBuilder and BigText
   cuts. Still refused until probed: JSON `\uD800` escapes, Base64 UTF-16 decode, .NET strings from the sidecar, stream reads
   (BC raises on some, per `moduleinfo.rs`).
3. **Unaudited readers refuse.** `Deref` and `as_str` are `#[track_caller]`. On a text with `lone`, they record the first
   caller in a per-thread slot. The VM checks the slot when a builtin returns and at the end of a statement, only in a session
   that has built a lone-surrogate text. A set slot raises `Signal::Unsupported("a text with half of a surrogate pair read by
   <operation> (not pinned)")`. So any operation not listed in item 4 refuses, as today, instead of using U+FFFD.
4. **Audited readers, each with its pin in a code comment:** `StrLen`, `T[i]` read, `foreach`, `CopyStr`, `DelStr`,
   `StrPos`, equality and `<>` by exact units, join, `Format` of a Text, `OutStream.WriteText` per encoding
   (`StTEncodings.al:249-265`, `P129StreamWriteText.al`, `P132OutStreamUnicode.al`, `StreamWrittenText.al`), XML write error
   (`XmlDocReadWrite.al:813-835`), JSON write (`W7JsonStreams.al`), BigText write (codeunit 80150), `UpperCase` and
   `LowerCase` (`CrTextLeads.al:414-439`). Everything else stays refused through item 3: sort and culture compare, filters,
   record keys and field writes to the store, `Evaluate`, `Message`/`Error` text, the .NET wire, the value codec of the
   compile cache (it already refuses what it cannot encode).
5. **Order of work.**
   1. Refuse the silent U+FFFD sites now (separate fix, red test first).
   2. `Txt` side list, normalization and the reader check, with unit tests; no behavior change. Measure `perf_vm` and
      `perf_budget` before and after.
   3. Sources (item 2) and audited readers (item 4), one corpus file at a time, red then green.
   4. Probes for readers the corpus does not pin yet, before they leave item 3's refusal.

## Consequences

- The 26 cases per major can run. The per-case work after step 2 is the audited reader each test needs.
- Valid text keeps its UTF-8 form, its ASCII fast paths and its speed. No seam changes: `Value::Text(Txt)` stays.
- A test that reaches an unaudited reader still refuses, with the operation named in the reason.
- Positions stay O(n) on non-ASCII text, as today.
- If later work needs exact UTF-16 everywhere (option b), this ADR does not block it: the `Txt` API above is the first step.

## Conformance

- Unit tests in `al-builtins/src/text.rs`: normalization, positions, `units()` round trip, the reader check.
- A test that every `Value::Text` built from UTF-16 units goes through `Txt::from_units` (no `from_utf16_lossy` in the
  text builders of `al-builtins` and `al-vm`).
- The corpus ratchet: the 26 cases leave `metrics/<major>/not-passing.txt` as they pass.
- `perf_vm` and `perf_budget` within their caps.
