# ADR-051: `--tdd`: a test that calls a procedure that does not exist yet runs against a stub

- Status: Accepted (owner, 2026-10-21: all of v2's kinds; the flagging in "Owner design" is fixed)
- Date: 2026-10-21
- Related: ADR-009 (an optional looser mode needs an ADR: this is it), ADR-011 (IR), ADR-016 (owner decision
  2026-10-21 that 3.0.0 needs v2's features), ADR-028 (serve), ADR-029 (incremental compile), ADR-048 (debug),
  ADR-050 (`--affected`)
- Parity row: SEL-10 (`--tdd`)

## Context

In test-first work, a developer writes the test before the procedure it calls. BC does not compile such a test, so
the developer sees a compile error instead of a red test. v2's `--tdd` generates the missing members so the test
runs and fails red. Today AL Runner exits 2 on `--tdd` and answers an error on the serve field `tdd`.

### Owner design (fixed, 2026-10-21)

- Only with `--tdd` on the command line, or `"tdd": true` in serve.
- A procedure or member a test calls that does not exist is generated as a stub. The stub returns the default value
  of its return type (0, '', false, an empty date, record or list). It has no side effects and never raises. `var`
  parameters stay unchanged.
- Every test that reached a stub is flagged "stubbed", whether it passed or failed.
- A stubbed pass still counts as a pass: status `pass`, the flag, and `generatedStubs`.
- A stubbed fail is a normal fail, with the flag and the stub names.
- The summary counts stubbed tests apart.
- The output field is `generatedStubs`, as v2.

### What v2 does

All references are to the `v2` branch.

- **Flags.** `--tdd` (help in `ProgramSupport/CliText.cs` around 925-938, `--guide` "TDD MODE" around 336-430).
  Refused with `--dap` (`Program.cs` around 803) and with `--watch --affected` (around 704). Works with `--watch`
  and `--jobs`. Serve: `"tdd": bool` on `runTests` (`ServerProtocol.cs` around 323); a server started with `--tdd`
  defaults to it; `execute` ignores it.
- **What is generated** (`TddGeneration.cs`, `TddGeneration.PackageMembers.cs`, `TddPrecompiledStub.cs`,
  `TddCrossBundle.cs`): missing procedures (AL0132) on source codeunits; a new overload when no overload has the
  argument count (AL0126); missing fields that a test assigns; enum values; missing codeunits, tables and enums
  (AL0185) with the first free id. For package objects: a stub codeunit, a table extension or an enum extension in
  the test's compile. All by rewriting AL syntax trees in memory, then compiling; up to 3 recompile passes.
- **Types.** Inferred from use for fixed-width types (Integer, BigInteger, Decimal, Boolean, Date, DateTime, Time,
  Duration, Guid, Enum). A call passed to a `Variant` parameter takes the type of the other `Variant` arguments.
  Text, Code, Record and Codeunit return types are refused. Parameters are rendered `Arg1: Integer`, never `var`.
- **Refused** (`Fixtures/Tdd/TddRefusedTests.Codeunit.al`, `Fixtures/TddCallShapes/RefusedTests.Codeunit.al`): a call
  as a bare statement (`Target.DoThing();`); both sides unknown (`Rec."Bar" := GetUnknownValue();`,
  `AreEqual(Target.ExpectedOf(1), Target.ActualOf(1))`); a Text expected value; `Variant` siblings that disagree; a
  builtin method with the wrong argument count. A refused object is excluded: each of its tests fails with
  `errorKind: "compile"` and `--tdd: <Obj> did not compile — <first diagnostic>`.
- **Which tests reached a stub** is read statically from the call graph (`TddCallGraph*.cs`): a branch that never
  ran still counts.
- **Output.** Per test (`--output-json` and serve `test` lines, `Reporter.cs` around 911, `ServerProtocol.cs` around
  459), only when not empty: `"generatedStubs": ["Calc: procedure \"DoubleIt\"(Arg1: Integer): Integer"]`, the form
  `<Object>: <kind> <signature>`. Status unchanged. No summary fields; the exit code follows the results. Text: per
  test `reaches generated stub(s): ...`, and a closing block on stderr:
  `--tdd: generated 2 member(s) this run:` with the list, then `--tdd: 2 test(s) reach generated stubs this run:`
  with `Codeunit.Test (fail): <stub>` lines.
- **Tests.** `TddModeTests`, `TddCallShapeTests`, `TddResultAnnotationTests`, `ServerTddTests`, `TddWatchTests`,
  `TddMissingObject*Tests`, `TddPackageMember*Tests`, `TddRerunLimit*Tests` and others.
- **Tools.** Neither ALchemist nor LethAL is known to send `tdd`.

## Decision

1. **Surface.** `alrunner test --tdd`. Serve: `"tdd": true` on `runTests`; `alrunner serve --tdd` makes it the
   default and `"tdd": false` turns it off for one request, as v2. On `execute`, `tdd` earns a `warnings` entry.
2. **Which calls become stubs.** With `--tdd`, the checker (`al-sema`) turns these diagnostics at a call site into a
   stub site instead of an error:
   - AL0132 for a procedure call on a variable of a codeunit, table (record), page or report, and on `Rec`,
     `CurrPage` or the object's own scope;
   - AL0118 for a call of an unknown procedure without a receiver (a local procedure of the same object);
   - AL0126 when no overload of the procedure has the call's argument count (a stub overload; an existing overload
     with the right count but other types is an ordinary compile error, as in BC).

   The target object may be in the workspace or in a dependency app; a stub procedure lives at the call site, so
   no object is changed for it (fields, enum values and objects: decision 10). Call sites are taken in the test apps (apps that declare a test codeunit). Call sites in
   the app under test stay compile errors (owner, 2026-10-21). Every other diagnostic stays an error, as without `--tdd`.
3. **The return type comes from the use.** In this order:
   - a bare statement `Target.DoThing();`: no return value (nothing reads it, so any type gives the same run; v2
     refused it);
   - the right side of `:=`: the type of the target;
   - a condition (`if`, `while`, `until`, `not`, `and`, `or`, `xor`): Boolean;
   - an argument to a parameter of a known type: that type; to a `Variant` parameter: the type of the other `Variant`
     arguments of the call when they agree (v2's rule);
   - `exit(...)`: the return type of the routine;
   - a comparison or arithmetic with an operand of a known type: that type;
   - a `case` selector: the type of the case labels;
   - `Format(...)` or text concatenation: Text.

   Text and Code are allowed (the default `''` is the same for every length; v2 refused them). A record type gives an
   empty record of that table; a List or Dictionary type an empty one; any other type the value an uninitialized
   variable of that type has, as the VM already gives it (no new rule). When no rule gives one type, the site is
   refused (decision 7).
4. **The stub at run time.** The IR gets one additive expression, `Expr::TddStub { stub, args, ty }`. It evaluates
   its arguments left to right, as a call does (an argument may call real code), and yields the default value of
   `ty`. It writes nothing, raises no event, runs no trigger, never raises an error, and leaves `var` arguments
   unchanged. The program table `stubs` holds each stub's signature for the report.
5. **"Reached" is observed at run time.** The session notes each stub id that runs during a test (additive session
   state, like the assumption ids of flagged passes). A test is stubbed when at least one stub ran while it ran,
   including in its handlers, subscribers and helper codeunits. This is exact: a stub in a branch that did not run
   does not flag the test. v2 decided this statically. The migration guide notes the difference.
6. **Signatures** use v2's form: `<Object>: procedure "<Name>"(Arg1: <Type>, ...): <Type>`, the argument types from
   the argument expressions, never `var`. Two sites that need different return types give two entries.
7. **What is refused**, statically, as a lowering reason of the routine that holds the site (rule 4 of the project):
   the routine's tests are reported not run with the reason, and every other test runs.
   - the return type cannot be inferred: `--tdd: cannot infer the return type of Target.DoThing (both sides are
     unknown)` and similar reasons for disagreeing `Variant` siblings and a call whose result is used as a receiver
     (`Target.GetRec().Insert()`);
   - a field, enum value or object whose type cannot be inferred (decision 10), or an object with no free id;
   - a page control on a `TestPage`, and missing pages, reports, queries, xmlports and interfaces (as v2);
   - a missing method of a builtin type (`Text`, `List`, `JsonObject`, ...): it is a typo, not a procedure to write;
   - a call on an interface variable: a stub would have to stand for every implementation.
8. **Output.**
   - Per test (`--output-json` and serve `test` lines): `"stubbed": true` and `"generatedStubs": [...]` (the stubs
     that ran during the test), both only when the test is stubbed. `status` stays the test's own outcome.
   - Summary (`--output-json` and serve `summary`): `"stubbed": {"passed": N, "failed": N, "error": N}` and
     `"generatedStubs": [...]` (every stub of the program, reached or not), only with `--tdd`.
   - Text: per test `reaches generated stub(s): ...`, as v2; the totals line `Tests: 10 total, 8 passed (2 stubbed),
     2 failed (1 stubbed), 0 not run`; v2's closing block on stderr.
   - JUnit: a `stubbed` property with the stub names on each stubbed test.
   - Exit code: follows the outcomes, as v2. A run whose only failures are stubbed exits 1, and a run of stubbed
     passes exits 0, because a stubbed pass counts as a pass (owner design).
   - The stubbed flag and the flagged-pass flag (unprobed assumptions) are independent; a test can have both.
9. **Compile and caches.** `tdd` is part of the warm-slot key (ADR-028). Stubs exist only in the workspace IR; the
   compile cache of the dependency apps is unchanged. An incremental compile (ADR-029) finds the stub sites of the
   objects it lowers. Adding the real procedure is a declaration change and compiles the workspace again, after
   which the stub is gone and the test is no longer flagged.
10. **Fields, enum values and objects (all of v2's kinds, owner 2026-10-21).** These are declarations, not call
    sites, so the checker adds stub declarations to the workspace compile. Nothing is written to disk and nothing
    from a dependency app is changed.
    - **A missing field** (AL0132 on a record field, read or written in a test app): a stub field in a stub table
      extension of that table (also of a dependency table, which is re-lowered as for any workspace extension,
      ADR-022). Field class Normal. The type comes from the use, by the rules of decision 3 (`Rec.F := 5`,
      `Validate`, `SetRange`, `TestField`, a comparison). It is a real column: a write is stored and read back, as
      in v2. Text and Code get the largest length (2048), because the real length is not known yet; a longer write
      raises `Signal::Unsupported` ("--tdd: the length of the stub field F is unknown"), never a made-up overflow
      error. The test is flagged either way.
    - **A missing enum value** (`Enum::"E"::NewValue`, also of a dependency enum): a stub member in a stub enum
      extension, ordinal the first free one above the highest existing ordinal.
    - **A missing overload** (AL0126): decision 2 already covers it.
    - **A missing object** (AL0185) of kind codeunit, table or enum, named by a variable declaration or an
      `Enum::`/`Codeunit::`/`Database::` reference in a test app, when no app in reach declares that name: a stub
      object with the first free id from the test app's `idRanges` (as v2). A codeunit stub has no procedures; calls
      on it are stub procedures (decision 2). A table stub has one primary key field `"TDD Key"` (Integer,
      AutoIncrement, as v2) plus its stub fields. An enum stub has the stub members its references need, ordinals
      from 0 in source order. Refused when a same-named object exists but is out of reach (another namespace, a
      missing dependency) or no id is free.
    - **Flagging** stays the owner design: a test is stubbed when, while it ran, a stub procedure ran, a stub field
      was read or written, a stub enum value was used, or a stub object was instantiated. Entries in
      `generatedStubs`: `<Table>: field "<Name>": <Type>`, `<Enum>: value "<Name>"`, `<kind> "<Name>" (id N)`, in
      v2's `<Object>: <kind> <signature>` form.
11. **No change without stubs.** With `--tdd` and no stub site, the program is byte-for-byte the program without
    `--tdd` (checked with the compile dump).

## Combinations

- `--affected --tdd`: allowed (ADR-050 decision 8; v2 refused it). There is no `--watch` in 3.0.0 (ADR-049 rejected).
- `debug --tdd`: allowed (ADR-048; v2 refused it). A stub has no statements to step into.
- `--tdd` with `--impact-of` and `impact-index build`: refused (exit 2). Those compare Microsoft's tests and have
  nothing to stub.

## Test plan (red, then green)

Fixtures in `crates/alrunner-cli/tests/fixtures/tdd/`, re-created from v2's `Tdd` and `TddCallShapes` fixtures.
Tests in `crates/alrunner-cli/tests/tdd.rs` and unit tests in `al-sema` for the type rule.

1. `--tdd` today exits 2 (red). A test calling a missing `DoubleIt` that asserts 4 fails, stubbed, with
   `generatedStubs`.
2. A test whose assertion holds against the default (`AreEqual(0, Target.Count(), '')`) passes, stubbed, counted
   as a pass and in `stubbed.passed`.
3. Each type rule of decision 3, one test per rule; Text and Code return ''; a record return is empty.
4. A bare-statement call runs and flags the test.
5. `var` argument unchanged; an argument that calls real code runs it once.
6. A stub in a branch that does not run does not flag the test; one in a handler or subscriber does.
7. AL0126: a stub overload next to a real procedure; a same-count overload with other types stays a compile error.
8. A stub on a dependency app's codeunit.
9. Each refusal of decision 7 gives the test not run with the reason, and the other tests run.
10. Without `--tdd` the same fixtures give the compile errors of today.
11. With `--tdd` on a project without missing procedures the compile dump equals the dump without it.
12. Serve: `tdd: true`, a daemon `--tdd` with `tdd: false`, `execute` warning.
13. Serve: add the real procedure, the next request is no longer stubbed.
14. A stub field: written and read back; type from each use; a 2049-character write refused; on a dependency table.
15. A stub enum value on a workspace enum and on a dependency enum; ordinals.
16. Stub codeunit, table (with `"TDD Key"`) and enum objects; the free-id rule; an out-of-reach name refused.
17. A test that only touches a stub field or enum value is flagged.
18. Text totals, JSON summary, JUnit property, exit codes.

## Size

About 9 agent-days: 2 for the checker (diagnostics to stub sites, the type rule, the refusal reasons); 1 for the IR
node, lowering and the per-test record in the VM; 3 for fields, enum values and missing objects (stub declarations,
dependency extensions, free ids, flagging); 1 for output (text, JSON, serve, JUnit); 2 for fixtures, tests and docs
(user guide, migration guide, parity row SEL-10).

## Consequences

- An additive IR expression and additive session state; no seam changes.
- Stub procedures live at the call site, so a dependency app needs no stub codeunit, unlike v2's precompiled stubs
  and overlays. Stub fields and enum values of a dependency object are workspace extensions, as any extension is.
- The flag is exact (observed), so a stubbed pass means a stub really ran.

## Owner decisions (2026-10-21)

1. Scope: all of v2's kinds (procedures, overloads, fields, enum values, missing codeunits, tables and enums).
2. Call sites: test apps only.
3. A run whose failures are all stubbed exits 1, as v2.
4. Summary `generatedStubs`: every stub of the program.

## Decided without owner review (2026-10-21, slice 4: decision 10)

- **Two-pass compile.** The checker notes the declarations a test app needs; they are written as AL source into one
  extra file of that app (`<tdd>/stubs.al`, in memory only) and the apps are compiled again, at most 3 more times (a
  stub table's fields are found once the table exists). A compile without needs is not repeated (decision 11).
- **Names and ids.** Table and enum extensions are named `"TDD Stub <id>"`. Every id (objects, extensions, the fields of
  a table extension) is the first free one of the test app's `idRanges`; an app without `idRanges` gets no
  declaration, and its sites stay refused.
- **Flagging granularity.** A use of a stub field or enum value marks the statement that holds it
  (`ir::Stmt::TddMark`, as `Stmt::Assume` marks an assumption); a stub object marks the routines that declare a
  parameter or local of its type. A global variable of a stub type does not flag by itself (changed in slice 5,
  below).
- **Length.** An assignment and `Validate` check the 2048-character limit of a stub Text or Code field
  (`ir::Expr::TddFit`); other writes (a `FieldRef`, `TransferFields`) keep the declared length's error (changed in
  slice 5, below).
- **Signature types.** A Text or Code stub field is listed as `Text` or `Code`, without the length.
- **Serve.** An incremental compile of a compile with stub declarations is refused; the request compiles completely
  and declares the stubs again from the new sources, so a stub that is no longer needed goes.

## Decided without owner review (2026-10-21, slice 5: follow-ups)

- **Globals.** A statement that names a global variable whose type is a stub object (codeunit, table, enum) is marked
  as a use of that object, so every test that touches the global is stubbed, never a clean pass. A test that does not
  name the global is not flagged by it.
- **Bare field names.** A bare name that is not called, in a trigger or procedure of a table or table extension of a
  test app, that nothing supplies (no variable, field, system variable, platform function or named section) is a stub
  field site of the table, as `Rec.F` is. Field triggers and other sections are left out, as for AL0118.
- **Dynamic writes.** A text longer than the 2048 characters of a stub Text or Code field is refused with the `--tdd`
  length reason also through `FieldRef.Value`, `FieldRef.Validate`, `Validate` by field number and `TransferFields`
  (`ir::Field::tdd_length_unknown`, read by the VM before it converts the value).

## Decided without owner review (2026-10-22, field arguments)

- **Validate and SetRange.** A bare name as the first argument of `Rec.Validate(F, ...)` or `Rec.SetRange(F, ...)`, in a
  test app, that is no field of the record's table and no local or global variable, is a stub field site, as `Rec.F`
  is. Its type comes from the value arguments by the Variant rule of decision 3 (`Validate(F, 5)` and
  `SetRange(F, 1, 10)` give Integer; `SetRange(F)` alone gives no type and is refused). The other methods that take a
  field argument followed (next section).
- Test: `crates/alrunner-cli/tests/tdd.rs`, `a_field_named_by_validate_or_setrange_is_a_stub_field`.

## Decided without owner review (2026-10-22, other field arguments and v2 parity)

- **Other field-argument methods.** The field argument of `TestField`, `SetFilter`, `ModifyAll`, `FieldNo` and
  `FieldCaption` (the first argument) and of `CalcFields` and `SetCurrentKey` (every argument) is a stub field site, as
  for `Validate` and `SetRange`. The type comes from the value arguments only: the value of `Validate`, `TestField`
  (an `ErrorInfo` argument is skipped) and `ModifyAll`, the bounds of `SetRange`, and the substitution values of
  `SetFilter` (`SetFilter(F, '>%1', 5)` gives Integer; the filter text says nothing about the field's type). Values of
  different types are refused; a value that is itself an untyped stub is refused as both sides unknown. A method without
  a value (`FieldNo(F)`, `FieldCaption(F)`, `SetCurrentKey(F)`, `TestField(F)`, `SetFilter(F, 'RED')`) gives no type:
  another use of the field in the test app must type it (the next compile then declares it and the method resolves to
  the stub field, flagging the test), or the routine is refused with "cannot infer the type of the stub field".
- **CalcFields.** `CalcFields` of a stub field is refused, also when another use typed and declared it: "--tdd:
  CalcFields of the stub field Item.F: a stub field is not a FlowField". A stub field is a Normal field.
- **Not extensible.** A stub value on a dependency enum that is not `Extensible = true`, and a stub field on a
  dependency table with `Extensible = false`, are refused ("--tdd: the enum "E" is not extensible: no stub value V can
  be added to it"), as v2 (`TddGeneration.PackageMembers.cs`). An object of the workspace always takes a stub: the
  developer adds the member to the object itself.
- **v2 kinds checked (branch `v2`).** v2 generates a subset of what AL Runner does (procedures only on codeunits,
  fields only from an assignment, no Text or Code). Its `K::V` and `"E"::V` enum values were already stub sites
  (unit test `an_enum_value_through_a_variable_or_the_bare_type_name_is_a_stub_value`). Not taken over: overloads keyed
  by argument types (decision 2 keeps a same-count overload with other types a compile error), stub codeunits for
  procedures of package objects (a stub here lives at the call site, decision 2), and generation into another source
  bundle of a run (AL Runner compiles all apps of a run together).
- Tests: `crates/alrunner-cli/tests/tdd.rs`, `a_field_named_by_other_field_methods_is_a_stub_field_and_calcfields_refuses`
  (fixture `tdd/field-args-more`); `al-sema` `check::tdd_decl` tests
  `a_dependency_enum_or_table_that_is_not_extensible_gets_no_stub_member`.

## Conformance

`crates/alrunner-cli/tests/tdd.rs`, the `al-sema` unit tests of the type rule, and the compile-dump comparison of
decision 11.
