# ADR-023: Page and report extension code that uses the base object's variables and procedures

- Status: Accepted (owner, 2026-10-14)
- Date: 2026-10-14
- Changes: how the lowering names a variable or procedure of the base page or base report in extension code, and one field of `Instance`. Additive in the IR (`Place::BaseGlobal`, `Expr::BaseInstance`); no seam is removed. It needs the owner's decision because it adds to the IR and to `Instance` (CLAUDE.md rule 5).

## Context

AL lets a page extension use what its base page marks for it: a `protected var` variable and a
procedure that is not `local`. A report extension reaches the `protected` members of its base
report. The runtime refuses both. Extension code lowers on the extension's own
object, whose globals and procedures are its own, so a name of the base page is "not assignable" or
unresolved. `Lowerer::base_member` (`al-sema/src/lower.rs`) detects the case for both kinds and gives it a
reason: ``\`X\` is a variable or procedure of the page it extends: page extension code on base page
members is not lowered yet`` (and the same with "report").

**How much it costs.** On BC 27.5 build 46862.56320, the 32 Microsoft test buckets together
(`alrunner test`, no test data, 40326 tests; recipe in `docs/alrunner-buckets.md`) have **2962 tests**
that stop at this reason. Three names account for all of them: `Item` (2931), `ServiceOrderNoVisible` (19),
`EnableOpenRelatedEntity` (12). One routine is the first stop of almost every one: the control
source expression `__source_1112` of `Mfg. ItemAvailByLocationLines`, a page extension in Base
Application whose field names `Item."Qty. on Component Lines"`, where `Item` is a `protected var
Item: Record Item` of the page `Item Availability by Location Lines`. Any test that opens that page
(or a page that hosts it as a part) stops there, Tests-SCM first (about 335 tests of that bucket).

**The report side** (scope added by the owner on issue 6). A report extension names `Window` and
`ExchangeAmtLCYToFCY` of the base report "Adjust Add. Reporting Currency" in Microsoft's "Mfg. Adjust
Add. Rep. Currency"; about 2500 Tests-ERM tests stop there (`docs/probes-needed.md`, "ReportExtension
and the base report's protected members", whose probe runs on the BC container machine and is not
part of this ADR's work).

**What exists today.**
- A page extension runs on an instance of its own (`Session::page_instance`, `al-vm/src/pages.rs`),
  with the globals of the extension. It shares three slots with the page's instance: `Rec`, `xRec`
  and `CurrPage` (they hold the same `Rc` handle).
- A *control* of an extension that names a base variable already works: `Binding::BaseVar(slot)`
  reads and writes the slot of the base page's instance, "whoever owns the control" (`al-ir`). It
  was built for report request pages (`reports/TestRequestPageControlErrorName.al`, documented in
  `docs/probes-needed.md` under "Request pages"). Only code is missing: statements and expressions
  in extension triggers and procedures, and control source expressions that are not a bare name
  (`Item."Qty. on Component Lines"`, `Format(BaseVar)`).
- A report extension that changes a run gets an instance of its own per run
  (`Session::report_prepare`, `al-vm/src/reports.rs`). Its record variables for the base data
  items are *aliases*: `ReportExt::aliases` copies the base item's record handle into an extension
  slot. That shares reference-like values only. A `protected var` of a value type (Integer, Text,
  Decimal) or a `Dialog` cannot be shared by copying the handle, which is why the aliasing does not
  extend to the Tests-ERM case.
- The checker (`al-sema/src/check.rs`, `ObjKind::PageExtension` and `ReportExtension` scopes) puts `Rec` and `CurrPage`
  into the extension's scope and nothing of the base page's `var` section or procedures.

## What the corpus pins

Everything about the *extension's own* instance, and nothing about base access:

| Pin | Where |
|---|---|
| An extension's global lives on the extension's instance: assigned in the extension's `OnOpenPage`, read by a control it declares, rewritten by that control's `OnValidate` | `pages/extensions/TestPageExtensionControlTrapped_Page.al:63-82`, `TestPageExtensionControlBinding_Page.al:56-72` |
| Extension code uses `Rec` (the page's record) for reads and writes, and its own local procedures | `testpage/TestPageExtensionActionInvoke_Page.al:70-91`, `pages/extensions/TestPageExtensionPageTriggers_Page.al:35-60`, `pages/extensions/TestModifiedControlValidateTriggers_Page.al:77-150`, `codeunits/TestCodeCoverageTableExtension.al:71-85` |
| An extension procedure publishes its own event | `events/extension-events/ALTExtEvtFixtures.al:88-96` |
| A control of a reportextension that names a `protected var` of the base report binds to the base report's variable (read, write, parameters text); a bare name that is no variable of the base is refused | `reports/TestRequestPageControlErrorName.al` (Ext5803Handler, cases 21-24) |
| Page and extension triggers run base first, then the extension (assumed for pages, pinned for reports) | `docs/probes-needed.md` "Page extensions" |

No file in the corpus contains `protected var`, and none has a page extension that calls a
procedure of its base page or reads a base variable from code. So the behavior below is **not
pinned**; it is what the AL language reference states and what Base Application's own source relies
on, and it needs probes on real BC before it is built (section "Probes").

## Options

**A. One instance for the page and its extensions.** The extension's globals are appended to the
page's global slots, and extension routines run on the page's `Instance`. Base variables are plain
slots of the same instance. Rejected: slot numbers of an extension then depend on every other
extension of the page that is loaded in the run (they are appended in application order), so the
lowering of one extension would depend on the set of apps, and the compile cache (ADR-022) could
no longer keep a per-object IR. It also changes what `Binding::Var(slot)` and `owner` mean for every
page that has extensions today (the 44 passing extension tests in `pageextension*`).

**B. Keep one instance per extension; give it a link to the base instance (recommended).** The
extension's own instance, globals and lowering stay as they are. Code that names a base variable
lowers to a new place that reads the base instance's slot; a call to a base procedure runs on the
base instance. This is the model `Binding::BaseVar` already uses for controls, extended from
controls to code.

**C. Alias the handles, as `ReportExt::aliases` does today.** Rejected as the general answer: it works for records and other reference values and not for value types (above), and a write by the base after the alias was made is invisible for those.

**D. Copy base variables into the extension on each call and write them back.** Rejected: two
extensions of one page, a base trigger and the extension would each work on copies; a value changed
by a nested call is lost or overwritten. It is also slow for record and list variables.

## Decision (proposed): option B

1. **Names.** In a page or report extension, a name that the extension does not declare resolves, in this
   order, to: the extension's own variable or procedure; then a `protected var` variable of the base
   page; then, for a page, a procedure of the base page that is not `local`; for a report, only members marked `protected` (the owner's statement on issue 6: whether a public procedure of a base report is also callable is probe R2 below); then the builtins (the order AL
   uses; to be confirmed by probe 6). A name of a `local` member of the base (a page or report), or one that is
   declared in both, is refused with a reason that says so. The checker puts the base object's
   visible variables and procedures into the extension's scope with their types, for `PageExtension` and `ReportExtension`.
2. **IR (additive).**
   - `Place::BaseGlobal(usize)`: the global slot `n` of the *base object's instance*. It is
     valid only in routines of a page or report extension; the lowering never emits it elsewhere. Reading and
     writing work as for `Place::Global`, including `Index` and `Field` projections, record
     variables (ADR-021 `TableGlobals` live on the shared `RecordState`, so nothing changes) and
     `var` arguments.
   - `Expr::BaseInstance`: evaluates to the base object's instance (page or report). `Expr::Call { instance:
     Some(BaseInstance), object: <base page>, routine }` calls a base procedure on it.
   The slot number of a base variable is its position in the base object's globals, the number
   `Binding::BaseVar` uses today, computed from the base's syntax (declaration order), not from
   the base's lowered object, so extension lowering stays independent of the base's lowering
   (they run on different threads).
3. **VM.** `Instance` gets `base: Option<Weak<RefCell<Instance>>>` (`None` for everything that is
   not a page extension's instance). `page_instance` sets it when it builds each extension's
   instance, next to the `Rec`, `xRec` and `CurrPage` sharing it does today, and `adopt_instance`
   keeps it; `report_prepare` sets it on the extension instance it builds for a run, next to the
   aliases (the base is the report instance of that run). A report run that has no extension
   builds nothing new. `Place::BaseGlobal` and `Expr::BaseInstance` read it from the running frame's
   instance; a missing or dropped base is an internal error with a readable text, never a
   panic. A `Weak` link adds no `Rc` cycle (the page instance already points to its `PageState` through
   `CurrPage`, which points back to the extension instances).
4. **Demand-driven compile.** A reference to a base variable or procedure records a call edge to
   the base routine (for a procedure) and keeps the base page at the level the extension is
   used at (`Demand::activate` already raises an extended page's extensions and an extension's
   target at one level). Nothing else in demand changes. This is the part most likely to need
   care: the 2962 tests above reach the base through a *dependency* app.
5. **Refusals stay explicit.** Refused, each with its own reason: access to a `local` member; a
   name declared by two extensions of the same page (not a base member); a base procedure that is a
   `[TryFunction]`, an event publisher or has `by var` record parameters of a type the call path
   refuses today (same reasons as a call within one object); `this` in an extension; a table extension
   (see "Out of scope"); anything probes 1 to 8 do not confirm.
6. **Triggers.** No change: base triggers run first and then the extension's, as now. A base
   variable that a base trigger set is what the extension's trigger reads.

## Consequences

- Up to 2962 tests of Microsoft's buckets on the page side and about 2500 on the report side (without test data) can run past these reasons. How many
  pass is another question: most then meet the next refusal (`Report.ObjectId`, `Format of a Record`,
  the `DocumentEntries` reportextensions; `docs/alrunner-buckets.md`).
- An extension's lowering depends on the base page's *syntax* (names, modifiers, declaration order)
  but not on its lowering. The compile cache (ADR-022) must hash the base page's source into the
  extension's key, which it already needs for `Binding::BaseVar` and for control merging.
- `Place` and `Expr` each get one variant. Code that matches them exhaustively (the checker of the
  call graph in `al-testrun/src/baseline.rs` `tables_written`, the lowering tests, the VM's `read`
  and `write`) gets one more arm; the compiler finds each.
- Cost when unused: one `Option<Weak>` per `Instance`; no check on the hot path of any other place.

## Probes (real BC; run on the main machine, not by an agent)

A corpus test file per row, in the style of `tableglobals/` (ADR-021): one claim per test, cited
by the implementation.

1. A page extension reads a base `protected var` (Integer, Text, Boolean, Record) that the base's `OnOpenPage` set.
2. A page extension writes a base `protected var`; the base's `OnAfterGetRecord` (or a base action) reads the new value.
3. Two extensions of one page: one writes, the other reads; and both read a value the base sets.
4. A page extension calls a non-local base procedure with a return value, with a `var` parameter, and one that changes a base variable.
5. A base `protected var` of type Record: the extension sets filters through it; the base sees them. Same for a `List` and a `Dictionary` (reference types).
6. Name resolution: an extension variable with the name of a `protected var` of the base (AL0 error or shadowing?), and an extension procedure with the name of a base procedure.
7. The state before the page runs: an extension's `OnInit` reads a base variable the base's `OnInit` has (or has not) set; which `OnInit` runs first.
8. `protected procedure` of the base: callable from the extension? And a base `local` variable: the compile error text (to refuse with the same text).
9. A control source expression that names a base variable (`Item."Qty. on Component Lines"`, `Format(BaseVar)`), through a TestPage, for a field of the base page's record and for a field that is not.
10. A base procedure called from an extension that runs a base trigger and reads `CurrPage` inside it.

Report side (the first is the one already listed in `docs/probes-needed.md`, run on the BC container machine):

R1. A base report sets a `protected var` in `OnPreReport`, the extension reads it in a trigger, writes it, and the base reads the write back; a protected procedure of the base called from an extension trigger and from an extension procedure.
R2. A public (non-protected) procedure of the base report from the extension, and a non-protected variable (a compile error, with its text).
R3. A `protected var` of type Dialog, Record and List in the base report, used from the extension (`Window.Open`, `Update`, `Close`).
R4. Two reportextensions of one report that both change the run: today refused; whether they share the base variable (they would, under this ADR).

## Pinned (2026-10-14, BC 27.5 container; corpus `tests/al-language/handlers/` and `reportextensiontrigger/`)

`base_access::PINNED` holds P1, P2, P4, P5, P9, P10, R1, R3, citing
`pages/extensions/TestPageExtensionBaseAccessContracts.al` (275, 288, 310, 326),
`pages/TestPageExtensionBaseMembersContracts.al` (197) and
`reports/extensions/TestReportExtensionBaseMembersContracts.al` (256).
- P6: an extension variable named like a base variable is a separate variable (the extension's own
  wins; base keeps its value). An extension procedure named like a base procedure is compile error
  AL0440: refused. A `local` base procedure or a base variable that is not `protected` is AL0161
  (`'<name>' is inaccessible due to its protection level`): refused with that text, because BC
  never runs such code.
- R2: a public base report procedure is callable, like a protected one.
- Several extensions share the base's variables; triggers run base first, then the extensions in
  object id order. Not probed: order across apps.
- The refusal "more than one reportextension changes the run" stays. Running several needs `Report::extension`
  (one object) and `Run::ext` to become lists, the data item placement of
  `TestReportExtensionSamePlaceContracts.al` (later extension nearest the anchor), and a rule for
  request pages; that changes the IR seam and is a separate ADR.

## Decided by the owner (2026-10-14)

1. **Option B accepted**: instance link, `Place::BaseGlobal`, `Expr::BaseInstance`.
2. **Probes and build run in parallel.** The plumbing is built now. Every access that no probe pins
   stays refused, with a reason that names the probe (`al-sema/src/base_access.rs`, `PINNED`). A
   confirmed probe turns its case on in a commit that cites the corpus test.
3. **Scope**: page extensions and report extensions. Table extensions stay out (refused as today).
   This is recorded in `docs/owner-review.md` as not reviewed by the owner.
4. **Order**: both sides use one mechanism; the probe results decide which cases turn on first.

## Implementation notes

- The checker puts the base object's `protected var` variables and its visible procedures into the
  extension's scope only while the mechanism is in use (`base_access::scope_enabled`: a probe is
  pinned, or `AL_UNPINNED_BASE_ACCESS` is set). Before that the compile reaches what it reached
  before. A variable resolves to `Res::BaseGlobal`; a procedure resolves to `Res::Proc` of the base
  routine, so the existing call edge reaches the base routine and nothing else in demand changes.
- Gates (all pinned now, see above; the text is kept for the refusal reasons): `P1/P2` variable access on a page, `P4` base procedure call, `P5` a record, list or
  other reference variable, `P9` a control's source expression, `R1` variable or procedure of a
  report, `R3` the reference types on a report. `P6` (a name declared twice) and `P8` (a `local`
  member, a variable that is not `protected`), `R2` (a public report procedure) are refusals that
  stay until the probe gives the compile error text.
- `AL_UNPINNED_BASE_ACCESS=1` (or a list of probe ids) turns the cases on without a probe. It is for
  development and for measuring how many tests move. A run with it is not a conformance result.
- The slot of a base variable is computed from the base's syntax (`base_access::slot_of`); a test
  compares it with the slot the base's own lowering gives.

## Decisions that were needed

1. **Accept option B** (instance link, `Place::BaseGlobal`, `Expr::BaseInstance`), or ask for A (one merged instance) with the cache cost described above.
2. **Probes first, or build first.** Everything in "Decision" except the refusals is unpinned. Build first, record what is assumed in `docs/probes-needed.md` and refuse nothing the language reference allows; or probe first (about ten tests) and build exactly what they pin.
3. **Scope.** Page and report extensions, as here; or also table extensions in the same step (`protected var` of a table; ADR-021 already gives the table's globals a lifetime on the record variable, which would be the base instance to link to).
4. **Report extension order.** Build the report side first (about 2500 Tests-ERM tests, one probe waiting) or the page side first (2962 tests across all buckets, probes 1 to 10 below). The mechanism is the same; the order only decides which probes are needed first.

## Out of scope

Table extensions (a tableextension has the same problem for the table's `protected var`), `Codeunit` access modifiers, and `internal` members across apps.

## Conformance

On acceptance: the probe tests above (corpus), the existing `an_extension_naming_a_variable_of_its_base_page_is_refused_with_that_reason` in `al-sema/src/lower.rs` replaced by tests that lower and run the same code, one VM test per decision 3 rule in `al-vm/tests/pageext.rs` and `al-vm/tests/reportext_calls.rs`, and the 2962-test measurement repeated with the recipe in `docs/alrunner-buckets.md` (report the new refusal reasons).
