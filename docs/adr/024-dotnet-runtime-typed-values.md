# ADR-024: DotNet values whose type is known only at run time

- Status: Accepted (owner, 2026-10-14): option A. Build deferred until the blockers in front of these tests are gone; probes go into the existing OnPrem corpus app (`corpus/tests/al-language-onprem`, target OnPrem), so decision 2 below is moot.
- Date: 2026-10-14
- Amends (on acceptance): ADR-014 section 4 (the `foreach` bullet) and section 8 item 7 (gaps)

## Decisions needed (owner, one line each)

1. **Type at the point of use, at run time (recommended), or a static type table at compile time?** Yes to the
   first means the sidecar is never started to compile (ADR-014 section 3 stays true) and one IR node is added.
2. **Add an `OnPrem` probe app to the corpus repo?** The corpus app is `"target": "Cloud"`
   (`corpus/tests/al-language/app.json:63`), so no corpus file can declare `DotNet` (AL0296). Without an
   `OnPrem` app, hard rule 3 can never be met for any `DotNet` behaviour, and every row below stays a
   documentation-only assumption.
3. **Until that probe app exists, run the primitive, stream and `foreach` paths on documented assumptions
   (recommended), or refuse them until probed?** Primitive operands use the marshalling table that
   assignments already use, so they add no new assumption. Object-valued `Format`, `foreach` element
   conversion and null results do.

## Context

Branch `adr-dotnet-operand`, main at 369aff9. Four refusal texts stop many tests (BC 27.5 build 46862.48827,
`alrunner test`, default flags, test data on, 4 threads; the number is tests whose first refusal it is):

| Refusal | Tests-ERM | NP Retail (npcore) | Tests-SMB |
|---|---:|---:|---:|
| `a DotNet value passed to builtin System.CopyStream#N` | 561 | 345 | 5 |
| `<expr> is a DotNet value used as an operand` | 363 | 356 | 19 |
| `a DotNet value passed to builtin System.Format#N` | 419 | 17 | 219 |
| `foreach over a DotNet value is refused` | 101 | 59 | 24 |

(The Format and CopyStream figures in the task text, 118 and 546, differ from these because the folded ranking
shows only the first reason per group; the figures here are counted from the JUnit of the run. NP Retail is
`~/Documents/Repos/community/external/npcore-partners`, run as
`alrunner test Test --dep Application --bc-version 27.5.46862.48827 --test-data`: 4813 tests, 219 passed,
4594 not run.) Company-Initialize no longer stops on the operand case; it stops on a `SetFilter` placeholder.
The Tests-SMB Office Add-in group (`Uri.Scheme`) still lists the operand case, and also
`OfficeHostType.OutlookItemRead`, which policy refuses (see "Refusals that stay").

### What stands behind the counts

Counted from the refusal text of each not-run test. Alias names are as the AL declares them; CLR types are
from the `dotnet { }` blocks. Only member names are quoted; no Microsoft or NaviPartner code is in this
repository.

**Operand (`ERM / NP / SMB` tests):**

| Expression (alias.member) | CLR result | Tests |
|---|---|---:|
| `Array.Length()` (System.Array) | int | 0 / 258 / 0, +2 NP in `RSA` |
| `Queue.Count` (System.Collections.Queue) | int | 150 / 3 / 0 |
| `BindingInfo.Fields.Count()` (Office add-in type, a list) | int, two member steps | 66 / 0 / 0 |
| `MatchCollection.Count` | int | 52 / 0 / 0 |
| `BinaryReader.BaseStream().Position()` | long | 0 / 38 / 0 |
| `File.Exists(...)` (System.IO.File) | bool | 39 / 20 / 0 (policy refuses the type next) |
| `JsonTextReader.ValueType.ToString()` | string | 0 / 17 / 0 |
| `DateTimeOffset.LocalDateTime` minus `.Offset` | DateTime, TimeSpan | 17 / 0 / 7 |
| `Uri.Scheme` | string | 12 / 14 / 6 |
| `XmlNode.Name`, `.HasChildNodes`, `XmlNodeList.Count` | string, bool, int | about 15 / 0 / 6 |
| `NumberStyles.A + NumberStyles.B + ...` | flags enum, `+` | 8 / 0 / 0 |
| one-offs (`XmlReader.Read()`, `OuterXml.Length()`, `SheetNames.Count`) | bool, int | 5 / 0 / 0 |

**Builtin argument:**

| Site (member names only) | Builtin | ERM / NP / SMB |
|---|---|---:|
| `ZipArchiveEntry.Open()` result as target of `CopyStream` (Data Compression `AddEntry`) | CopyStream, OutStream parameter, result of unknown static type | 561 / 39 / 5 |
| a `DotNet MemoryStream` variable as target of `CopyStream` (Base64 `ToBase64(InStream)`) | CopyStream, **declared** alias, not a run-time type | 0 / 306 / 0 |
| `UserInfo.ObjectId()` (Azure AD plan code) and `DataColumn...DataType` (Business Chart) | Format, Variant parameter | 419 / 17 / 219 |
| a `DotNet` result to `Dialog.Error` (4 tests) and `Record.Validate` (1) | other parameter types | 5 / 0 / 0 |

**foreach:** `Regex` `MatchCollection`, `GroupCollection`, `CaptureCollection` (NP 59), `String.Split` result
(`GetSafeFileName`, 13 in ERM), request-page helper collections (ERM 55 / SMB 24), `XSD Parser` nodes (26),
Excel sheet lists (35), Data Compression entry list (2).

### What BC does

- The AL compiler binds each alias at compile time by loading the assembly (ADR-014 section 3, `docs/history/dotnet-generic.md`
  section 1.1). It therefore knows the CLR type of every member access and picks the AL type, operator and
  conversion statically. These expressions compile in BC because it can reflect over the assembly.
- At run time the emitted C# is late-bound reflection through `NavDotNet` (`docs/history/dotnet-generic.md` section 1.2,
  item 6): `Text` is `string`, `Decimal` is `decimal`, `Date`, `Time`, `DateTime` are `DateTime`, `InStream`,
  `OutStream` and `Stream` are `System.IO.Stream`. The reverse direction (a .NET `int` seen as an AL `Integer`,
  a `Stream` accepted where an `OutStream` is expected) is what the compiler's type mapping does. It is **not
  pinned** anywhere in the corpus (see "Corpus pins and probes").
- A `Stream`-typed `DotNet` value passed to an `OutStream` or `InStream` parameter compiles in BC (Microsoft's
  code does it), so BC has a stream adapter in both directions.

### What the runtime already has

- The sidecar already returns typed wire values (`tools/dotnet-sidecar/Marshal.cs`): an `int` is `i4`, a `long`
  `i8`, a `string` `s`, a `bool` `b`, a `DateTime` `dt`, a `TimeSpan` `ts`, and `wire_to_value` in
  `al-vm/src/dotnet/mod.rs` turns them into `Value::Integer`, `BigInteger`, `Text`, `Boolean`, `DateTime`,
  `Duration`. Everything else is a `Value::DotNet` handle that carries its CLR type name.
- So **at run time the value already knows its type.** Only lowering refuses, because the checker types a
  `DotNet` member result as `Ty::DotNet("")` (`al-sema/src/check.rs`, `member`) and `no_dotnet_operand`
  (`al-sema/src/lower.rs`) refuses it.
- `Expr::DotNetCoerce(RtType, e)` already stores such a result into a typed slot, with `fit` as the rule (exact
  kind, numbers widen). The same trust in the marshalling table already exists for assignments, `exit`, `if`
  and a few builtin parameters.

### Measured: what lifting the four refusals gives

A throwaway spike (not committed; lowering let a `DotNet` operand, a `Format` argument and a `foreach` through,
`CopyStream` unchanged) shows where the same tests stop next:

| Bucket | Passed before | Passed after | Where the tests stop next |
|---|---:|---:|---|
| Tests-ERM | 557 | 557 | `FileUpload.CreateInStream` (570), `IsEmpty on Application Area Setup` (366, from the install baseline), `Format of a Record` (118), platform table column (66) |
| NP Retail | 219 | 221 | report extension on a base report (317), `Active Session` column 4/5 (184), `Format of a Record` (56), `RecordId.GetRecord` (48), `JsonObject.Keys` (52) |

Lifting this does not by itself move the pass count much: **the refusals are static and reach-based**. One
refused routine takes every test that can reach it, whether or not the test executes that line. For example,
364 of the 395 tests that stopped in the Azure AD plan code next stop on an install-baseline row; the run does
not show that those tests needed the Azure AD code. The value
of the change is that the next blockers become visible and that a test stops only where it really runs into
something. Expect the pass counts to follow as the next blockers fall.

## Options

### A. Type at the point of use, at run time (recommended)

Lowering no longer asks what the CLR type is. The value carries it.

- Operand, condition, for-loop bound, case scrutinee, procedure argument, `exit`, assignment: a result with no
  static type is wrapped in a settle step that checks the run-time kind against what the context needs. An
  expression whose operands include an untyped result has an untyped result for arithmetic and the static type
  Boolean for comparison, `not`, `and`, `or`.
- Builtin argument: a scalar parameter takes the settled value (`fit`); a `Variant` parameter (`Format`,
  `Message`, `StrSubstNo`...) takes it as is; a `var` parameter is refused.
- `InStream` / `OutStream` parameter (`CopyStream`): the argument stays a `Value::DotNet`, and the VM copies
  bytes to or from the sidecar (below).
- `foreach`: the VM iterates a `Value::DotNet` with `GetEnumerator`, `MoveNext`, `Current` as ordinary host
  calls, and settles each `Current` into the loop variable with `fit`.
- Cost: one new IR node, a checker change so the type of an expression is carried (`Ty::DotNet("")`
  propagates through arithmetic), a VM branch in `Stmt::Foreach`, two host calls and two sidecar ops for
  streams. About 400 lines plus tests. No compile-time work, no cache change.
- Risk: a refusal that was static (before the test started) can now come mid-run, when the value is null or an
  object (hard rule 4 allows `Signal::Unsupported` for what can only be found mid-run; a test refused midway is
  reported not run since `npcore-fixes`). The same change turns many static refusals into dynamic ones, which
  stops one unreachable routine from blocking hundreds of tests.

### B. Static type table from the sidecar's reflection at compile time, cached

At compile time ask the sidecar for the CLR signature of every member the code reaches; lower with the exact
AL type, as BC's compiler does.

- Matches BC's compile-time typing and reports an unknown member before the test starts.
- Costs: compile must start the sidecar (about 50 to 65 ms warm, 1 s or more cold, ADR-014 section 6), which
  breaks "compilation never starts the sidecar" (ADR-014 section 3), core purity for WebAssembly hosts
  (ADR-016) and a run without .NET (ADR-014 section 11). It needs a full type walk (`BindingInfo.Fields.Count()`
  is two steps over an add-in type, `Array.GetValue` returns `object`), the overload-dependent return types of
  `NavAutomationHelper`, and the assemblies a user adds with `--dotnet-assembly-dir`, known only at run. The
  cache key of ADR-022 would have to include the assembly set. About 55 assemblies and 587 aliases in Microsoft's
  apps alone.
- Rejected as the first step. It can still be added later as diagnostics only (option C).

### C. Hand-written signatures for hot BCL members in `dotnet_sigs.rs`

`al-builtins/src/dotnet_sigs.rs` already gives registry types a static result type. Extend it with the CLR
signatures of the members in the tables above (`Array.Length`, `Queue.Count`, `MatchCollection.Count`,
`Uri.Scheme`, ...).

- Works without .NET, and gives early typing for the common members.
- Costs: a table to maintain, incomplete (the long tail stays untyped), and each entry is a claim about a CLR
  signature. It does not solve `CopyStream`, `foreach` or the unknown tail, so it cannot be the only mechanism.
- Recommended only as an optimisation on top of A, if a run without .NET or a profile needs it.

### D. Keep refusing

Tests that reach Microsoft's `DotNet` code stay not run. Microsoft's code cannot be changed (hard rule 7), so
this blocks about 1450 Tests-ERM tests, about 780 NP Retail tests and every other app that uses these base
codeunits. Not acceptable as the end state.

## Decision (proposed): option A, with option C as a later optimisation

### 1. Operands and typed contexts (`al-sema`, `al-ir`, `al-vm`)

- New IR node `Expr::DotNetOperand(Box<Expr>)` (additive). The VM evaluates the inner expression and returns:
  - the value, if it is one of Boolean, Integer, BigInteger, Decimal, Byte, Char, Text, Code, Date, Time,
    DateTime, Duration, Guid (the kinds `wire_to_value` produces);
  - `Signal::Unsupported("a null .NET result used as an operand: ...")` for `Value::Null`;
  - `Signal::Unsupported("a DotNet object of CLR type X used as an operand: ...")` for any other
    `Value::DotNet`, the CLR type taken from the handle, so a probe author can read it.
- `no_dotnet_operand` stops refusing and instead wraps the operand. The ternary arm and `in` set cases
  keep their own refusal (see below).
- The checker types an arithmetic expression with an untyped operand as `Ty::DotNet("")`, and a comparison,
  `not`, `and`, `or`, `in` as Boolean. Contexts that need a type wrap with the existing
  `DotNetCoerce(slot, e)`: assignment, `exit`, `if`/`while`/`until` (done), and now for-loop bounds, `case`
  scrutinee, `Text[i]` index, and the argument to an AL procedure with a scalar parameter.
- The result type of an operator is the one the VM's operator gives for the settled kinds, so `Count - 1` is
  Integer arithmetic and `Count / 2` Decimal, as for AL variables. No new operator rules.

### 2. Builtin arguments

- In `lower.rs` (call to a builtin, the loop that tests `Ty::DotNet(alias)` per parameter): a scalar parameter
  (Text, Code, Integer, BigInteger, Decimal, Boolean, Char, Byte, Date, Time, DateTime, Duration, Guid) gets
  `DotNetCoerce(rt, arg)`. A `Variant` parameter gets `DotNetOperand(arg)`. `DotNet` stays as today.
- `Format` of a settled primitive is the ordinary `Format`. `Format` of a `Value::DotNet` object, an enum or
  null is not pinned; see "Refusals that stay" and probe 3.

### 3. Streams: `CopyStream` and later stream builtins

- For a builtin parameter of type `InStream` or `OutStream` and an argument of any `DotNet` type (declared
  alias such as `DotNet MemoryStream`, or an untyped result such as `ZipArchiveEntry.Open()`), lowering passes
  the value unchanged. No static check of the CLR type is possible, so the sidecar checks at run time that the
  handle is a `System.IO.Stream`.
- `System.CopyStream#N` gets a VM-level branch (like `Session.BindSubscription`, `al-vm/src/lib.rs`): when the
  target is a host `Value::DotNet`, read the unread bytes of the AL `InStream` (position advances, as pinned by
  session/TestSystemExtended.al:94-128) and write them with `stream_write`; when the source is a host
  `Value::DotNet`, `stream_read` and append to the AL `OutStream`. The optional byte count is honoured.
- `al-host` gets two defaulted methods on `DotNetSession`: `stream_write(handle, &[u8])` and
  `stream_read(handle, max) -> Vec<u8>` (default: refuse "this host cannot copy a stream"). The sidecar
  gets the ops `stream_write` and `stream_read`; a handle that is not a `Stream` is refused with the CLR type
  in the reason. This is additive. `hello` compares the protocol version for equality, so the version is bumped and the host and
  the sidecar change together (the sidecar is built from `tools/dotnet-sidecar` in the same repository).
- This replaces the copy-in/copy-out bridge only for `CopyStream`. Passing an AL stream to a .NET method
  (`GZipStream.GZipStream(OutStream, mode)`) keeps ADR-014 section 4 unchanged.

### 4. `foreach`

- Lowering drops the `Ty::DotNet(_)` refusal; `Stmt::Foreach` is unchanged (`var`, `collection`, `body`).
- The VM, when `collection` evaluates to a `Value::DotNet`: `GetEnumerator()`, then loop `MoveNext()`;
  `Current` is settled into the loop variable's `RtType` with `fit` (a `DotNet`-typed variable keeps the
  handle; `Text` takes a `string`). An element that does not fit is `Signal::Unsupported` naming both kinds.
- The enumerator handle is released at loop end through the existing release queue (`Drop`, ADR-014
  section 2). Whether BC calls `Dispose` is open (ADR-014 section 13 item 3); the runtime calls nothing.
- A null collection, an exception in `MoveNext` (modified collection) and `break`/`exit` out of the loop
  follow the general rules of ADR-014 section 7: a .NET exception is `Signal::Unsupported` until pinned.

### 5. Seam classification

| Change | Additive or seam? |
|---|---|
| `Expr::DotNetOperand` in `al-ir` | Additive (new IR node, as `DotNetCoerce` was). Bumps the compile-cache runtime build (ADR-022) |
| `Ty::DotNet("")` propagating through arithmetic in `al-sema` | Inside the checker, no seam |
| `DotNetSession::stream_read` / `stream_write` in `al-host` | Additive (defaulted methods) |
| Two ops in the sidecar protocol | Additive; protocol version bumped (`hello` requires equality), sidecar and host change together |
| `Value`, `RtType`, `Store`, `Ctx` | Unchanged |
| ADR-014 section 4 `foreach` bullet and section 8 item 7 | Behaviour change by this ADR; no seam moves |

No seam changes, so under hard rule 5 the owner's acceptance is needed only because it relaxes two ADR-014
rules (static refusal of operands and of `foreach`).

## Refusals that stay

1. **A null `DotNet` result used as an operand or settled to a scalar.** What BC gives (`''`, `0`, an error)
   is not pinned. Runtime `Signal::Unsupported`.
2. **A `DotNet` object (not a primitive) as an operand,** including `=` between two objects and `DotNet`
   enum `+`/`=` (`NumberStyles.Number + NumberStyles.AllowCurrencySymbol`, 8 tests in TypeHelper). Runtime
   refusal naming the CLR type.
3. **`Format` of a `DotNet` object or enum, or null.** The Business Chart code compares
   `Format(DataColumn.DataType)` with CLR type names, which is consistent with `ToString()` but does not pin it.
   Refused until probe 3, or run on the documented assumption if question 3 is answered yes.
4. **A `DotNet` value in a conditional (ternary) expression, `in` sets, `X[i]` on a `DotNet` result,**
   `var` arguments of a builtin. Unchanged.
5. **Policy refusals (ADR-014 section 8) now become the visible reason** for tests that stopped earlier on the
   operand rule: `System.IO.File.Exists` (59 tests), `DateTimeOffset.Now` (17), `System.Drawing.Bitmap`,
   `OfficeHostType.OutlookItemRead` (a server class that is neither registered nor classified pure),
   `MailAddress` (network). These stay refused, except `MailAddress`: see the owner decision below.
6. **Observed, not part of this ADR:** `Encoding.UTF8` reports `member not found` in 24 NP Retail tests;
   boxed primitives from `Array.GetValue(I)` assigned to `DotNet Byte` need a rule for `fit`
   (`Value::Byte` into a `T::DotNet` slot). Both need their own change.

## Corpus pins and probes

### What the corpus pins (and what it does not)

No corpus test contains `DotNet` (the corpus app is `Cloud`). The nearest pins:

| Behaviour | Corpus |
|---|---|
| `CopyStream(OutStream, InStream)` between blobs copies the whole unread stream and advances the source | session/TestSystemExtended.al:94-128; empty stream without error: session/TestSystemExtended.al:132-157 |
| `foreach` over `List` and `Dictionary`: order, empty, `break`, body semantics | collections/TestForeachContracts.al:16-194 |
| `DateTime + Duration` | session/TestDateTimeContracts.al:50-75 |
| BC accepts `Format(Variant, 0, 2)` on a `DotNet` variant (`Variant.IsDotNet`) | `_fixtures/Assert.al:493` (a fixture; no test passes a `DotNet` value) |

### Probes needed (an `OnPrem` probe app on a BC 27.5 container; written down, not run)

Added to `docs/probes-needed.md`, "DotNet values typed at run time (ADR-024)". Each probe reads the kind
through `Variant` checks (`IsInteger`, `IsText`, ...) and `Format`:

1. **Kind seen by AL for each CLR result:** `int`, `long`, `short`, `byte`, `char`, `float`, `double`,
   `decimal`, `bool`, `string`, `DateTime`, `TimeSpan`, `Guid`, an enum, `null` string, `null` object.
   `Count - 1`, `Count / 2`, `Length() - 1` as a for bound, `Scheme = 'HTTPS'` (case), `Name <> ''`.
2. **A null `string` result as an operand:** `s = ''`, `StrLen(s)`, `s + 'x'`, as a `Variant`.
3. **`Format` of a `DotNet` object:** `System.Type`, an enum, `StringBuilder`, a null object; with
   `Format(x, 0, 2)`. Compared with `ToString()`.
4. **`DotNet` enum operators:** flags `+`, `=` between enums, enum against `Integer` or an `Option`.
5. **`foreach` over:** a `string[]` (`Array`), `MatchCollection`, `Queue`, `List<string>`, `Dictionary`
   (element type), a `string` (chars), a null collection (error text and code), a collection modified in the
   loop (exception text), a loop variable of the wrong type, `Text` over `int` elements, and whether the
   enumerator is disposed at the end, on `break`, on `exit` and on an error.
6. **`CopyStream` with a `DotNet` stream:** target `MemoryStream` (content via `ToArray`, `Position` after),
   source `ZipArchiveEntry.Open()` or `MemoryStream` into a blob, with and without the byte count, a source
   stream at a non-zero position, a read-only target (exception text), and a non-`Stream` object as the target
   (compile error or run-time error text).
7. **`=` and `<>` between `DotNet` objects:** same object, two equal strings in `object` variables, `IsNull`.
8. **`DateTimeOffset.LocalDateTime - Offset`:** AL type of the result, and `DateTime` minus `Duration`.

Until a probe lands, the runtime takes the behaviour from the marshalling table (ADR-014 section 4, the
`fit` rule) and refuses everything under "Refusals that stay".

## Consequences

- About 1450 Tests-ERM and 780 NP Retail tests stop being refused by these four rules and reach the next
  blocker; the spike shows the pass count rises little until those fall (table above).
- Compile time, the compile cache and WebAssembly hosts are unaffected. A run without .NET still refuses at
  the first host call.
- Some refusals move from "before the test starts" to "when the line runs". A test with side effects before
  such a line is reported not run (since `npcore-fixes`), never failed.
- `Expr::DotNetOperand` changes the IR; the cache key of ADR-022 includes the runtime build, so existing
  entries are rebuilt once.
- The sidecar protocol grows by two ops. `tools/dotnet-sidecar/Dispatch.cs` and the `DotNetSession` default
  methods must stay in step; `hello` requires equal protocol versions, so a stale sidecar is refused at start-up.

## Conformance

On implementation (not part of this docs-only change):

- Unit tests in `al-sema` (lowering of operand, `Format`, for bound, `foreach` produce the new nodes) and
  `al-vm` with the deterministic test host in `al-host` (primitive settle, null refused, object refused with the
  CLR type, `CopyStream` both directions, `foreach` over a scripted enumerator).
- A sidecar test for `stream_write` / `stream_read` on a `MemoryStream` and on a non-stream handle.
- The Tests-ERM, Tests-SMB and NP Retail figures above are the before numbers; `--baseline-diff` is the check.
- `tools/demand_vs_eager.sh run`, since lowering and the checker change.

## Probe results, round 6 (2026-10-14, BC 27.5, notes for the build)

Probed in the OnPrem corpus app: `tests/al-language-onprem/dotnet/TestDotNetValueKindsContracts.al` (codeunit 61250,
nine tests). The runtime still refuses all nine; this section is what the build must match. The facts are listed in
`docs/probes-needed.md`, "Probe results". Where they change this ADR:

- **Decision 3 is answered by the probe**, not by an assumption: the kind map of the marshalling table is right
  (`int`/`short` Integer, `long` BigInteger, `byte` Byte, `char` Char, `float`/`double`/`decimal` Decimal, `bool`
  Boolean, `string` Text, `TimeSpan` Duration, `Guid`, `DateTime`). An enum stays a DotNet object.
- **"Refusals that stay" 1 (null result).** A null `string` is an empty Variant: `= ''` true, `Format` `''`, `+ 'x'`
  gives `'x'`. It is no longer refused for a string. A null object in `Format` or `StrLen` is an UNCATCHABLE error
  ("Unexpected CLR exception thrown.: System.ArgumentNullException"; `asserterror` does not catch it). The run-time settle
  step cannot tell a null string from a null object by value alone, so the sidecar reply must carry the declared CLR
  return type for a null (additive wire change), or the null stays refused.
- **Refusals that stay 2 and 3 (objects, enums, `Format`).** `Format` of an object is `ToString()` (Type, enum member name,
  StringBuilder content; the length and format-number arguments change nothing). Enum `+` adds the numbers, `=` and
  `<>` compare, `=` between two class objects does not compile (AL0175), so it needs no run-time rule.
- **String compare** against a literal is case sensitive: the settled operand must compare as a CLR string, unlike a
  Text variable.
- **foreach** error texts and codes are pinned (see probes-needed): a null collection is `DotNetVariableNotCreated`, a
  collection modified in the loop `DotNetInvoke:InvalidOperation` after the first element, a wrong element kind
  `Argument` or `Conversion`. The `Current` conversion into a Text variable over int elements is an error, not a
  settle by `fit`.
- **CopyStream with a MemoryStream.** The whole MemoryStream is copied into an OutStream whatever its position, and its
  position is 0 afterwards; with a count it copies from the current position and leaves the position after the
  count. This replaces the sentence "reads the unread bytes" in decision 3 for the MemoryStream-to-AL direction.
- **Probe 4 (enum operators)** and **probe 7 (`=` between objects)** are done as listed; probe 8 (DateTimeOffset) too.

Nothing here is built: every item needs `Expr::DotNetOperand`, which is the build this ADR defers.

## Build notes, slice 1 (2026-10-14, branch adr-024)

The probe app `tests/al-language-onprem-dotnet/` (corpus `tests/al-language-onprem-dotnet/`, codeunit 61400) is a
local-container result: CI cannot compile it (it needs AL 18), so it has no oracle and is not in the corpus
submodule. Code comments cite it by corpus path.

- IR: `Expr::DotNetOperand(Box<Expr>, OperandUse)` (the ADR's node plus a use, because `+ = <>` accept a .NET enum,
  `< <= > >=` refuse a string, a `Variant` parameter takes an object as the handle and a scalar builtin parameter
  refuses a null) and `BinOp::DotNetEq` / `DotNetNe` (a CLR string compares by case). Additive.
- Wire: tag `ns` (`DotNetValue::NullString`), a null whose declared CLR type is `string`; protocol version 2.
  The VM keeps "the last result was a null string" on the session (`DotNetState::null_string`), cleared at the start
  of every settle step and DotNet member, because `Value::Null` cannot carry it (`Value` is unchanged).
- `DateTime.Now`, `UtcNow`, `Today`, `DateTimeOffset.Now`, `UtcNow` read the host clock (see `owner-review.md`).
- `Format` of an object is its `ToString()` through the sidecar (`Ctx::format_dotnet`, a defaulted method); of an
  enum its member name. Format numbers 0 and 2 only.
- Not done in slice 1: `CopyStream`, `foreach`, `StrLen`/`Format` of a null object (refused, uncatchable error needs a
  seam), a `DotNet DateTime` variable holding a by-value `DateTime` (`LocalTime := Stamp.LocalDateTime`).
- The compile cache stores IR only (no sidecar data); the new nodes change the executable hash, so old entries rebuild.

## Build notes, slices 2 and 3 (2026-10-14, branch adr-024)

- `CopyStream` (`Ctx::copy_stream_dotnet`, defaulted): AL `InStream` into a `DotNet` stream writes all unread bytes
  (`DotNetSession::stream_write`) and moves the AL position to the end; a byte count in this direction is refused
  (not pinned). A `DotNet` `MemoryStream` into an AL `OutStream` copies the whole stream whatever the position and
  leaves position 0, or with a count the bytes from the position, which then follows (`stream_read`). Reading any other
  stream type is refused by the sidecar (only `MemoryStream` is pinned). Two sidecar ops, protocol version 3.
  Both directions go through the policy first (a `FileStream` stays refused). Between two `DotNet` streams is refused.
- `foreach` over a host object: `GetEnumerator`, `MoveNext`, `Current` as host calls, the enumerator handle is released with
  the loop. Pinned errors are raised as AL errors: null collection (`DotNetVariableNotCreated`, text for a codeunit only,
  other object kinds are refused), collection modified (`DotNetInvoke:InvalidOperation`, only that .NET exception type),
  `Int32` into Text (`Argument`) and Text into Date (`Conversion`). Any other bad element or exception is a refusal.
  `foreach` over a `string` value (characters) is not built: a Text cannot be held in a `DotNet String` variable yet.

## Owner decision 2026-10-18: System.Net.Mail.MailAddress is allowed

The owner allows the .NET type `System.Net.Mail.MailAddress`, and only it. It parses text and does not touch the
network. The corpus pins it in `tests/al-language-onprem-dotnet/lt-af-dotnet/AfMailAddress.al` (constructor, `User`,
`Host`, `DisplayName`, `ToString`, error texts and codes `DotNetInvoke:Format` and `DotNetInvoke:Argument`).
`policy.txt` has a new rule kind `allow-type` (exact type, never a prefix) placed before `deny-type System.Net.`.
Every other `System.Net.` type stays refused, including `SmtpClient` and `MailMessage`.
