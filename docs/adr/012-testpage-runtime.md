# ADR-012: TestPage runtime (headless page objects and the test client)

- Status: Accepted (project owner, 2026-10-13)
- Date: 2026-10-13

## Context

Pages are the largest block of the corpus that no seam covers yet. Measured
with `python3 tools/testpage_survey.py corpus/tests/al-language` (static text
survey, a ceiling and not a prediction):

| Tier | `[Test]` procedures |
|---|---:|
| page-driven in total (30% of the 4836) | 1434 |
| core: `TestPage` on a page that lives in the workspace, no handler, no part | 807 |
| `[PageHandler]` / `[ModalPageHandler]`, `Page.Run*`, `RunObject` actions, dialogs | 236 |
| pages with `part(...)` / `TestPart` | 210 |
| Base App pages and request pages of reports (not in the workspace) | 176 |
| `RunPageBackgroundTask` | 5 |

Of the 807 core tests, 179 sit on pages with `AutoFormat*` / `DecimalPlaces` /
`BlankZero` / `BlankNumbers` (control text formatting), 58 call
`ALTFixtureCleanup` (blocked until table-event subscribers exist, an area
another agent owns) and 11 use `EventSubscriber` / `BindSubscription`.

Today every one of them stops at `type TestPage "X" is not lowered yet`
(`al-sema/src/lower.rs`, `fn rt`), or, when the test carries
`[HandlerFunctions]` (339 tests in the whole corpus), earlier at "handler
functions are not supported yet". The type checker already resolves the
members (`al-sema/src/check.rs`: controls, `Filter`, parts, actions) and the
catalog already has `TestPage.*`, `TestField.*`, `TestAction.*`, `TestPart.*`,
`TestFilter.*` signatures. What is missing is everything behind them:
lowering of page objects, a value for a page, and a runtime.

### What the corpus asks of a "TestPage"

A `TestPage` is not a renderer. It is a simulation of the **BC test client**
driving a page instance that runs in the same session and the same
transaction as the test. The corpus pins that client's behaviour to a very
fine grain; most of it was measured on real BC (the file headers say so).
The behaviours a first model must know:

| Behaviour | Corpus |
|---|---|
| `OpenEdit/OpenView/OpenNew` run `OnInit`/`OnOpenPage`, then read the first row of the view | `handlers/TestPageRecordTriggers.al:147` (`OnOpenPage_RunsBeforeThePageIsRead`), `testpageclose/TestPageClosingActions.al:11-14` (reopen shows the FIRST row) |
| `OpenNew` runs `OnNewRecord` before any field is set; `OnInsertRecord` returning false suppresses the insert | `TestPageRecordTriggers.al:32`, `:76` |
| `OK().Invoke()` persists a new row at once; `Close()` on a dirty new Card persists it too | `TestPageRecordTriggers.al:95`, `:125` |
| On a List the row is inserted once its key is typed, later controls are a Modify; on a Card the insert waits for Close/OK; `DelayedInsert` changes that | `handlers/TestPageBlankKeyInsert.al:1-25`, `handlers/TestPageTriggerEvents.al` (`NewAndInsertRecordEvents_PageDrivenInsert_FireForTheKeyOnly`) |
| `OnClosePage` runs on `Close()`; `OnAfterGetCurrRecord` runs on every move | `TestPageRecordTriggers.al:187`, `:205` |
| After `OK`/`Cancel`/`Close` the variable is closed; every call raises `The TestPage is not open.`; an `Open*` on an open variable raises `The TestPage is already open.`; a card/list has no `Cancel` (`The built-in action = Cancel is not found on the page.`) | `testpageclose/TestPageClosingActions.al:9-24`, `:338-339` |
| An error escaping `OnAfterGetRecord` or a control expression tears the page down; from then on every call raises `The TestPage is not open.` | `testpageteardown/TestPageTeardown.al:3-24` (85 tests) |
| `Visible` of a control is frozen at open, `Editable`/`Enabled` are live, a group's `Visible` is live | `handlers/TestPageControlPropertyLiveness_Tests.al:56`, `:73` |
| Control text: each type has its own en-US spelling (`Yes`/`No`, `1,234.50`, short dates, braced Guid, option captions); `AssertEquals` compares the control's text ordinally; `SetValue` of a typed value is spelled the same way and re-parsed | `handlers/TestPageFieldFormats_Tests.al:1-40` (107 tests), `handlers/TestPageExtended.al:179` |
| `GoToKey` returns false for a missing key; `FindFirstField(Field, Variant)` is type-sensitive (`'20'` does not match an Integer field) | `handlers/TestPageExtended.al:34`, `handlers/TestPageAdvanced.al:302` |
| A modal page opened by AL (`Page.RunModal`, a `RunObject` action, `Lookup`) is answered by the test's `[ModalPageHandler]`, synchronously, with a `TestPage` over the running page; OK/Cancel become the `RunModal` result (`LookupOK` in lookup mode); no handler raises `Unhandled UI: ModalPage` | `handlers/TestPageModalHandler_Tests.al:43`, `:83`, `:105`, `:132` |
| A page-driven save gets the full data-layer treatment (AutoIncrement, `SystemCreatedAt`...) | `pagesave/TestPageSaveDataLayer.al:1-30` |
| The platform raises page events (`OnOpenPageEvent`, `OnModifyRecordEvent`, ...) after the page's own trigger | `handlers/TestPageTriggerEvents.al:1-20` |

Constructs used by the 341 pages and 40 page extensions of the corpus (counted
inside page objects only): `field` 865, `action` 142, `repeater` 127, `part` 82,
`group` 66, `actionref` 11, `systemaction` 6; triggers `OnAction` 103,
`OnOpenPage` 73, `OnValidate` 43, `OnAfterGetCurrRecord` 19,
`OnQueryClosePage` 19, `OnAfterGetRecord` 14, `OnNewRecord` 12,
`OnClosePage` 11, `OnInsertRecord` 5, `OnDrillDown` 4, `OnLookup` 3,
`OnFindRecord` 2, `OnInit` 2, `OnAssistEdit` 2, `OnModifyRecord` 1; properties
`SourceTable`, `SourceTableTemporary`, `Editable`/`Visible`/`Enabled`
(constants and expressions), `DelayedInsert`, `AutoSplitKey`,
`InsertAllowed`, `SubPageLink` / `SubPageView`, `RunObject` /
`RunPageLink`, `Caption` / `CaptionClass`, `SaveValues`. `TestPage` members used (approximate text counts over the whole corpus):
`Close` 1020, `OpenEdit` 588, `OpenView` 403, `First` 209, `GoToKey` 192,
`GoToRecord` 155, `Next` 128, `OK` 100, `OpenNew` 84, `Filter` 62, `Last` 38,
`Cancel` 34; on controls `Value` 683, `SetValue` 323, `Invoke` 240,
`AssertEquals` 105, `Caption` 65, `Visible` 63, `Editable` 62, `Enabled` 51.

### What exists in the code that this has to meet

* `ir::Object` has `table: Option<Table>` for tables and nothing for pages;
  page routines lower as plain routines, but there is no `Rec`/`xRec` slot
  for a page (`table_of` only knows tables and table extensions) and no
  `CurrPage`.
* `RtType` / `Value` (ADR-010) have no page-shaped variant; `Library(String)`
  is the generic opaque-handle escape hatch.
* Record builtins reach the store through `Ctx::record` (`native.rs`),
  implemented in `al-vm/src/records.rs`. Anything that must call back into AL
  (a page trigger) has to live in `al-vm`.
* `blocked_reason` (`al-test-cli/src/runtime.rs`) decides "not run" from the
  **static call graph** (`Lowered::calls`). A page trigger is called by the
  runtime, not by AL, so it is invisible to that analysis unless something
  adds the edge. Without that, a test could pass the check, open a page and
  die inside an unsupported trigger at run time (hard rule 4).
* Another agent is adding `[HandlerFunctions]` for Message/Confirm/StrMenu
  (`wt-handlers`, `runtime.rs`). Page handlers are the same mechanism with a
  different key, so the two must share one registry.

## Decision (owner's answers)

The project owner accepted the design below and answered the open questions:

1. **Core design accepted as written**: IR page object and control tree,
   `Value::TestPage` / `Value::TestControl`, a `Ctx::page` hook, a client
   state machine.
2. **Scope: everything at once**, not only record-bound Card/List pages:
   control-text formatting (AutoFormat, DecimalPlaces, BlankZero,
   BlankNumbers), parts / `TestPart`, `[PageHandler]`, `[ModalPageHandler]`,
   `[RequestPageHandler]` (request pages are part of this model), `Page.Run`,
   `Page.RunModal` and `RunObject` actions. Still refused explicitly:
   anything the corpus does not pin, anything that needs another area
   (table events and `ALTFixtureCleanup` belong to the events agent), Base
   App pages outside the workspace, and `RunPageBackgroundTask` unless the
   corpus pins it. The "first increment" lists below are therefore the
   order of work, not the limit of scope.
3. **Culture**: fixed en-US in the VM.
4. **Location**: inside `al-vm` (`al-vm/src/pages.rs` and neighbours).
5. **Handlers** build on the registry merged in 22a3875
   (`al-vm/src/ui.rs`: `UiKind`, `UiScope`, `dispatch_ui`;
   `resolve_handlers` / `bind_handlers` in `al-test-cli/src/runtime.rs`).
   Events stay refused until the events branch lands. A run-time
   `Signal::Unsupported` is acceptable for what can only be discovered
   mid-test (an explicit "not run"); static refusal through the call graph is
   preferred wherever possible.

## Design

### 1. Shape: a page is data plus ordinary routines; TestPage is a client over a page instance

* **Lowering** turns a `page` object (and its `pageextension`s, merged the
  way table extensions are today) into `ir::Object.page: Option<ir::Page>`,
  a flat description of the control tree. All AL code stays ordinary
  `ir::Routine`s: triggers, procedures, and **one synthetic routine per
  expression-valued property** (`Visible = not HideIt;`,
  `AutoFormatExpression = RowFormat();`, `Editable = Rec.Flag;`) returning
  Boolean/Text, lowered in the page's scope. The runtime never interprets
  syntax.
* **Page instance** = an `Instance` (the existing globals container used for
  codeunits) whose globals are the page's `var` section plus the implicit
  `Rec` and `xRec` (new `rec_slot` / `xrec_slot` for pages, exactly as for
  tables). Triggers and property routines therefore run through the existing
  `Session::call(object, routine, instance, args)`; the VM interpreter does
  not change. `CurrPage` is lowered to builtins on the instance
  (`CurrPage.Update/Close/SaveRecord/SetSelectionFilter/...`), implemented by
  the page runtime; unimplemented members are refused at lowering.
* **The client** is a Rust module `al-vm/src/pages.rs` (like `records.rs`)
  holding the state machine: closed / open / torn down, mode (view/edit/new),
  the cursor on the source table (records through the existing store with the
  page's `SourceTableView`, filters and current key), the draft row of a
  new record, the dirty buffer, the validation-error list, the open-time
  snapshot of frozen properties. It is synchronous (ADR-002): `OpenEdit`
  returns after `OnOpenPage` and the first row load; a handler for a modal
  page runs inside `RunModal`.
* **Record writes use the record layer** (`insert/modify/validate` in
  `records.rs`), so table triggers, system fields, AutoIncrement and
  `Validate` behave as for AL code (`pagesave/TestPageSaveDataLayer.al`).

### 2. IR additions (al-ir)

```
Object.page: Option<Page>            // pages and page extensions (merged)
Page { page_type, source_table: Option<usize>, temporary: bool,
       props: PageProps { editable, insert_allowed, modify_allowed,
                          delete_allowed, delayed_insert, auto_split_key,
                          save_values, source_view, caption ... },
       controls: Vec<Control>,              // flat; `parent` index = tree
       triggers: PageTriggers { on_init, on_open_page, on_after_get_record,
                                on_after_get_curr_record, on_new_record,
                                on_insert_record, on_modify_record,
                                on_delete_record, on_query_close_page,
                                on_close_page, on_find_record, ... : Option<usize> } }
Control { name, kind: Field|Group|Repeater|Action|Part|SystemPart,
          parent: Option<usize>,
          binding: Field(source field pos) | Var(global slot) | None,
          caption: Static(String)|Expr(routine)|FromField,
          visible/editable/enabled: Const(bool)|Expr(routine)|Inherit,
          format: { decimal_places, blank_zero, blank_numbers,
                    auto_format_type, auto_format_expression },
          on_validate / on_lookup / on_drill_down / on_assist_edit / on_action: Option<routine> }
Expr::Control { page: Box<Expr>, control: usize }   // TP.Ctl, resolved statically
RtType::TestPage(page object) ; TestPart(page object) ; TestControl ; TestFilter(table)
```

Page-level and control-level facts the first increment does not understand
are **not** dropped: lowering records a reason on the routines that declare a
`TestPage` of that page (see section 6), so they are refused, not guessed.

### 3. Value additions (al-builtins)

`Value::TestPage(Rc<RefCell<PageState>>)` and
`Value::TestControl(Rc<ControlRef>)` (a control handle is `(page state,
control index)` and also stands for `TestField`, `TestAction`, `TestPart`,
`TestFilter`). Reference semantics, like `Codeunit`. `PageState` is plain
data (no VM types): page object, instance, mode, cursor, flags. The alternative
`RtType::Library("TestPage")` loses the page object index the default value
needs, so named variants are proposed (ADR-010 already lists "the Json/Xml/
stream/builder types" as variants).

Page builtins reach the VM through a new `Ctx::page(method, receiver, args)`
hook next to `Ctx::record`, implemented in `al-vm/src/pages.rs`.

### 4. Control text: one pure formatter

A pure function `control_text(value, &Format, &Culture) -> String` and its
inverse `parse_control_text` in `al-builtins` (not the VM): the spellings in
`TestPageFieldFormats_Tests.al:1-40`, and `AutoFormatExpression` parsing
(`<Precision,4:4><Standard Format,0>`). Culture is fixed en-US with the
space before AM/PM as U+0020 (the corpus test normalises both,
`TestPageFieldFormats_Tests.al:58`). It is unit-tested against the measured
values and is the only place that spells control text.

### 5. Property timing is data, not code

A table in `pages.rs` records, per property, when it is evaluated: `Visible`
of a control at open (frozen), `Editable`/`Enabled` live, group `Visible`
live (`TestPageControlPropertyLiveness_Tests.al`), `Caption`/format
expressions at row load (errors there tear the page down,
`TestPageTeardown.al`). Each entry cites its test.

### 6. Refusal is static (hard rule 4) and complete

Lowering gives a routine a reason when it **declares or reaches** a
`TestPage` / `TestPart` variable of a page that uses anything outside the
increment: the reason names the page and the feature ("page `X`: `part(...)`
is not supported yet"). Reaching is made true by adding call-graph edges
from every routine that declares a `TestPage` of page P to every routine of
P (and of its extensions), and from `Page.Run*`/`RunObject` targets, so the
existing `blocked_reason` sees page triggers and property routines. A feature
that can only be discovered at run time raises `Signal::Unsupported`, never a
default.

### 7. Handlers share one registry with the dialog handlers

`[PageHandler]` / `[ModalPageHandler]` bind by the page type of their
`TestPage` parameter (several per test), where Message/Confirm/StrMenu bind
by kind (one per kind). The session owns one registry filled from
`[HandlerFunctions]`; a modal open (`Page.RunModal`, `RunObject` action,
`Lookup`/`AssistEdit`) looks up the page handler, builds a `TestPage` over
the running instance, calls the handler, then closes the page with the
action the handler chose (`OK`, `Cancel`, `LookupOK`...) and returns that
result to the AL that opened it; no handler raises `Unhandled UI: ModalPage
...` (`TestPageModalHandler_Tests.al:105`). This needs the dialog agent's
registry to land first.

### 8. Page platform events

`OnOpenPageEvent`, `OnModifyRecordEvent`, ... are published by the page
lifecycle after the page's own trigger (`TestPageTriggerEvents.al`). The
page runtime calls a single `publish_page_event(page, name, args)` seam that
is a no-op until the events area (another agent) provides a registry; tests
that subscribe stay refused until then.

## The first increment (if this ADR is accepted)

In (the ceiling is about 560 of the core tests: 807 minus the 179, 58 and 11
above if they do not overlap; the achieved number will be lower because the
corpus pins close/teardown/list-window detail):

* Card and List pages on an in-workspace source table, `SourceTableTemporary`
  excluded, page extensions that add/modify fields and actions.
* `OpenEdit/OpenView/OpenNew`, `Close`, `First/Last/Next/Previous/New`,
  `GoToKey/GoToRecord`, `FindFirst/Next/PreviousField`, `Filter.SetFilter`,
  `Caption/Editable/Visible/Enabled`, `ValidationErrorCount`.
* Fields bound to `Rec.Field` and to page variables: `Value`, `SetValue`,
  `AssertEquals`, `AsInteger/AsDecimal/AsBoolean/AsDate/AsTime/AsDateTime`,
  `OnValidate`, `Lookup`/`DrillDown`/`AssistEdit` where the trigger is plain AL.
* Actions: `Invoke`, `Visible/Enabled`, `OnAction`, `OK()`, built-in
  `Cancel` refusal.
* Page triggers `OnInit`, `OnOpenPage`, `OnAfterGetRecord`,
  `OnAfterGetCurrRecord`, `OnNewRecord`, `OnInsertRecord`, `OnModifyRecord`,
  `OnDeleteRecord`, `OnQueryClosePage`, `OnClosePage`, `OnFindRecord`;
  `CurrPage.Update/SaveRecord/Close`.
* Closed / already-open / torn-down state machine.
* The control text formatter for Integer, BigInteger, Decimal (+ decimal
  places, blank rules), Boolean, Option/Enum, Date/Time/DateTime, Code, Text,
  Guid, Duration.

Out (refused explicitly, each with a reason naming the feature):

* `part(...)` and `TestPart` (210 tests; needs link filters and a child
  instance per part), `SourceTableTemporary` pages and `SaveValues`.
* `[PageHandler]` / `[ModalPageHandler]`, `Page.Run*`, `RunObject` actions
  (236): second increment, after the handler registry exists.
* Base App pages and request pages (176): outside the workspace; request pages
  belong with the report area.
* `RunPageBackgroundTask`, `Expand`, media fields, list-window loading,
  `CaptionClass` beyond a literal, page platform events.

## Alternatives considered

1. **Desugar a page to a codeunit and write the client in AL.** Rejected: the
   client's rules (frozen vs live properties, teardown, flush points) are host
   behaviour that AL cannot express, and every rule would still need to be
   written once, in a worse place.
2. **Put the client behind `Library("TestPage")` with no new variants.** Smaller
   diff in `value.rs`, but the page object index and control handles become
   strings; every access is a downcast. Kept as the fallback if the project
   owner wants no `Value`/`RtType` growth.
3. **A new crate `al-page`** (client + formatter, depending on a `PageHost`
   trait with `call`, record ops). Cleaner dependency picture, but the client
   needs record ops, the session's trigger machinery and the handler
   registry, all of which live in `al-vm`; the trait would mirror half of
   `Session`. The formatter alone can be its own module in `al-builtins`.
4. **Lazy evaluation of control properties on read.** Rejected: it would give
   live `Visible` where BC freezes it, and would raise expression errors at
   the read instead of tearing the page down at the load
   (`TestPageControlPropertyLiveness_Tests.al`, `TestPageTeardown.al`).

## Consequences

* A new subsystem (my estimate, unmeasured: a few thousand lines) across `al-sema` (page lowering),
  `al-ir`, `al-builtins` (formatter, values) and `al-vm` (client). The
  interpreter is untouched.
* `Value`, `RtType`, `ir::Object` and `Ctx` grow (seams, hence this ADR).
* The corpus pins client behaviour at the level of single messages, so the
  achieved pass count tracks how much of that detail is modelled; the
  honest-refusal rule keeps the unmodelled remainder "not run", never wrong.
* Pages become a second kind of object with an implicit `Rec`; the fix to
  `table_of` benefits page extensions and later report/request pages.
* Some core tests also depend on other areas (69 on events: 58 through
  `ALTFixtureCleanup`, 11 through subscribers; others on Format/Evaluate and
  further builtins), so the pass rate of this area will lag behind those
  merges.

## Open questions (answered, see Decision)

1. **Accept the IR/Value/Ctx growth** in sections 2-3, or prefer the
   `Library("TestPage")` fallback (alternative 2)?
2. **Fidelity budget.** The corpus pins roughly 85 + 27 + 107 tests on teardown
   rules, expression timing and control formatting alone. Model them all
   (large, but each is measured on real BC), or accept "not run" for the
   formatting and teardown files in the first increment and spend the effort
   on the 500-odd record-bound tests?
3. **Culture.** Fixed en-US for control text (matches the corpus), or a host
   capability in `al-host` now so another culture can be added later?
4. **Order of merges with the other agents.** The page runtime wants (a) the
   dialog-handler registry for modal pages and (b) the event registry for
   page platform events and for `ALTFixtureCleanup`. Should the page work wait
   for them, or land first behind explicit refusals?
5. **Where the client lives**: `al-vm/src/pages.rs` (proposed) or a separate
   crate (alternative 3).
6. **Request pages** (`TestRequestPage`, 158 `RequestPageHandler` uses): same
   model, or a separate ADR with the report area?
7. **Hard rule 4 and run-time refusals.** Static refusal by page feature is
   proposed; some features (an expression that errors, a trigger that opens
   another page) are only known at run time and would raise
   `Signal::Unsupported` mid-test. Is that acceptable when the test then
   reports "not run" rather than failing?

## Conformance

* `tools/testpage_survey.py` reproduces the table above from the corpus.
* A Rust test per client rule, each citing the corpus line it comes from
  (`file:line` in a comment), written red first.
* The formatter has table-driven unit tests whose rows are the measured
  values of `TestPageFieldFormats_Tests.al`.
* The corpus pass-rate ratchet (ADR-007): no AL test may newly fail.

## Decided without owner review (2026-10-13/14)

Page extensions in the test client (branch `pageext`). Each decision has an
entry in `docs/probes-needed.md` ("Page extensions") and a line in
`docs/owner-review.md`.

1. **Several extensions that declare one page trigger both run, in application
   order.** Options: refuse (the table-extension precedent), or run in
   application order. Chosen: run, because two corpus tests count the runs of
   two extensions' OnOpenPage and state no order
   (`pageextensiontrigger/TestPageExtQualifiedExtends_Tests.al`), and
   `TestModifiedControlValidateTriggers_Page.al` says the order is the
   platform's load order. Application order is: dependency app first, then
   object id.
2. **The page's own trigger runs before the extensions' copies.** No corpus
   test observes it. AL Runner v2 binds each extension to the page and BC's
   `RaiseOn<trigger>Async` runs the extension copies after the page's own
   (`docs/pageextension-binding.md` there).
3. **Only the void page triggers are supported in an extension** (OnOpenPage,
   OnClosePage, OnAfterGetRecord, OnAfterGetCurrRecord, OnNewRecord). The
   Boolean ones (OnQueryClosePage, OnInsertRecord, OnModifyRecord,
   OnDeleteRecord) are refused because how the answers combine is not pinned.
   OnInit, OnFindRecord and OnNextRecord are page-only in BC.
4. **When extensions `modify` one property, the later extension in application
   order wins**, for every property. Pinned for Caption only
   (`handlers/TestPageExtensionModifyCaption_Tests.al`).
5. **Application areas are read when the page opens.** A control the session's
   areas do not enable is not on the page. A control with no area of its own
   takes the page's, unless an extension declared it; with the areas
   restricted, a control with no area at all is removed. An actionref with no
   area follows its target. If the session's areas change while a page is
   open, the run is refused. The test runner empties the areas before each
   test method.
6. **`DrillDown`, `AssistEdit` and `Lookup` on a TestField run the control's
   trigger** (its own, or the one a `modify` block adds). Without a trigger,
   `DrillDown` raises the pinned platform error; the other two are refused.
7. **An actionref is a control.** Invoking it invokes its target. Its
   `Visible`, `Enabled` and `Caption` are refused.

Remaining TestPage-client gaps (branch `pages-misc`). Each has rows in `docs/probes-needed.md` and a line in
`docs/owner-review.md`.

8. **A `Page` variable is constructed at its first use, which runs OnInit once.** Options: leave procedure
   calls on a `Page` variable refused, or construct the page then. Chosen: construct, because
   `handlers/TestPageOnInit_Tests.al:188` pins "I, then the setter, then OnOpenPage". The open does not run OnInit again.
9. **A list that serves its rows from OnFindRecord/OnNextRecord is walked through the triggers.**
   First, Last, Next, Previous and GoToKey ask the triggers (`-`, `+`, +1, -1); a `CurrPage.Update` leaves the row
   unread until the next move. Options: keep refusing such pages (10 tests), or model the walk. Chosen: model it
   for the shapes the corpus asserts, and refuse reads the corpus does not pin.
10. **The row an action deleted on such a list** is replaced by the four-call sequence the corpus measured.
    The rule "the client knows other rows when its populated window holds more than the gone row" is an
    inference from four tests; a shape outside them is refused.
11. **`TestField.Lookup`** runs the control trigger, else the table field's OnLookup, else the TableRelation's lookup
    page. A value written back by a `true` OnLookup goes through the same path as `SetValue`.
12. **Page background tasks run synchronously, all of them, before the TestPage call returns**, in a read-only
    worker session. Options: refuse, or model. Chosen: model; the transaction behaviour pinned by
    `TestPageBgTaskTx_Tests.al` is not modelled and ends the run as unsupported.
13. **`asserterror` around a failing TestPage call** keeps its rollback-to-last-commit rule; a case where BC
    obviously keeps earlier uncommitted rows (a failed lookup) is refused when writes are pending, not
    changed (the transaction area is not this branch's).
### Request pages and FilterPageBuilder (branch `requestpages`)

Decided without owner review (2026-10-13/14). Each decision has an entry in
`docs/probes-needed.md` ("Request page", "FilterPageBuilder") and a line in
`docs/owner-review.md`.

Answer to open question 6 of this ADR: a request page is **the same client over
the report's own instance**, not a second model. `Object.page` of a report holds
the request page's control tree; its controls bind to the report's global
variables, and each data item is a control of kind `DataItem` bound to the
item's record. `TestRequestPage "R"` lowers to `RtType::TestPage(R)`;
`[RequestPageHandler]` is a `UiKind` of its own, bound by the report. The
reportextensions that extend a request page merge in like page extensions
(`Page.extensions`, `Control.owner`).

1. **A report with no `requestpage` section opens a request page of its data
   items' filters**, unless `UseRequestPage` is false, the caller passes
   `RequestWindow = false` or the instance called `UseRequestPage(false)`. The
   reports agent left this open (`record/TestTxModelReportRun.al:11, :60-61` say
   a report with a request page defaults to using it; no corpus report without
   a section is run with the page on). Options: refuse such a run, or open the
   page. Chosen: open it, because BC shows the data item filters for any report
   that is not `UseRequestPage = false`. Without a handler in the test the run
   is refused either way (decision 2), so no corpus test changes.
2. **A run that opens a request page with no `[RequestPageHandler]` for the
   report in the test is refused** (`Signal::Unsupported`). Options: raise
   `Unhandled UI: RequestPage` as page runs do, or refuse. Chosen: refuse,
   because no corpus test pins what the platform says.
3. **The data items' own views are in force before the request page opens and are
   not applied again after it.** The handler's filter would otherwise be wiped
   by the item's `DataItemTableView`. Pinned in a comment of
   `handlers/TestReportMultiLineDataItemView.al`, not asserted.
4. **`RunRequestPage` returns an XML text of this runtime's own layout**
   (`ReportParameters`, `Options`, `DataItems`), documented in
   `docs/probes-needed.md` with what the corpus pins. It takes the same text
   back as a seed and refuses any other text.
5. **`Invoke` on an action of a request page raises "The action with ID = N is
   not found on the page." with N derived by FNV-1a over the report id and the
   action name.** The formula is fitted to three corpus ids
   (`handlers/TestReportRequestPageAction.al:31-33`) and gives no other
   pinned id. If a probe contradicts it, replace `action_id` in
   `pages/request.rs`.
6. **A report that renders output (not `ProcessingOnly`, or a layout) is
   refused when its run reaches the point where output would be produced**,
   not when its variable is declared. This lets the corpus test that pins its
   request page (no plain OK) run. Every run of such a report is still refused
   unless the handler cancels or raises.
7. **`RunRequestPage` commits the transaction world it entered, as `Run`
   does.** No corpus test reads the data afterwards.
8. **FilterPageBuilder**: `RunModal` and `[FilterPageHandler]` are refused (no
   corpus test calls them; superseded 2026-10-29: filterpage/W3FilterPageBuilder.al and
   platform/lt-platform-members/PlTFilterPageBuilder.al pin a one-table builder, see
   `Session::ui_filter_page`); the default `PageCaption` is `Filter`; `AddRecord`
   and `AddRecordRef` copy the record; error sentences other than the two the
   corpus pins in full are filled in around the control name.
