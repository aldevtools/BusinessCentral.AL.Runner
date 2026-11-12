# ADR-075: Report data items from dependency extensions

- Status: Proposed
- Date: 2026-11-12
- Amends: [ADR-025](025-multiple-reportextensions-per-run.md), decision 2 (cross-app order) and decision 4 (first refusal)
- Related: [ADR-022](022-compile-cache.md)

## Context

A report extension can introduce a data item. A downstream extension in an app
that depends on its owner can refer to that item. The neutral repro in runner
issue #7878 passed on BC 27.5.46862.56996, 28.5.54151.57550 and
29.0.54011.57566. The consumer's report extension object ID is lower than the
owner's. The owner ran first, the consumer's OnAfterAfterGetRecord ran against
its row, and the serialized XML contained the consumer's column and the linked
siblings in their observed positions.

The proposed corpus pin in PR #1342 uses two separate reports in
`reports/extensions/TestReportDependencyDataItems.al`. The reversed-ID fixture
has owner 61144 in `ReportDependencyB.ReportExt.al` and consumer 61124 in
`ReportDependencyC.ReportExt.al`. The rising-ID fixture has owner 61186 in
`ReportDependencyRisingB.ReportExt.al` and consumer 61361 in
`ReportDependencyRisingC.ReportExt.al`. Both use the same XApp A/B/C dependency
topology and assert the same trigger trace and XML column value.

## Options

1. Keep ADR-025's refusal. This avoids changing the IR and leaves the measured
   dependent report unable to run.
2. Resolve the name when the VM opens the report. The VM would need to repeat
   application visibility and ambiguity checks and discover the table behind
   each foreign record before binding its fields.
3. Resolve the owner and table during semantic lowering and carry the typed
   reference into the VM. This gives checking, linking and execution the same
   object identity and makes cache validation explicit.

## Decision

Resolve during semantic lowering. Use the app IDs explicitly declared in the
consumer manifest's dependency list. `app_sees` describes transitive visibility
and does not establish a direct dependency. An owner in the same app, in an
unrelated app, or reachable only indirectly keeps the existing lowering refusal.

Support the measured foreign dataset references only when all these conditions
hold:

1. Exactly two extensions change the report run, in distinct apps.
2. The consumer app declares the owner app as a direct dependency.
3. A consumer `modify` resolves to a unique item declared by that owner.

The measured `modify` (with OnAfterAfterGetRecord) and `add` (columns)
target `AddedRow`, whose immediate parent belongs to the base report.
Sibling `addbefore` and `addafter` target `FooterRow`, whose immediate
parent is the owner's `AddedRow`. Apply the existing placement and implicit
link rules. For an explicit `DataItemLinkReference`, allow a name that resolves
to the foreign item that is the immediate parent. In both fixtures
`AfterFooter` names `AddedRow` and reads `after=2`. Runner PR #7912 supplies
the enclosing-item rule and its base and same-extension cases and lands first.

Keep the established order rules in `al-sema/src/lower/ext_order.rs`.
When the owner's object ID is lower, those rules already put it first.
When the consumer's object ID is lower, allow the additional owner-first
ordering only for the qualifying pair above. A reversed pair without the
qualifying `modify` keeps the current cross-app order refusal. Foreign
references with three or more extensions are refused for either ID order.
Reports without foreign references continue to use the established order rules.

A consumer variable with the owner item's name causes a lowering refusal.
Preserve the existing reason text for refused shapes. Also refuse ambiguous
items, unmeasured triggers and child placement, and `RequestFilterFields`
on a foreign modification during lowering.

The supported entry point is the static stream call
`Report.SaveAs(id, '', ReportFormat::Xml, OutStream)`, with no `RecordRef`
argument. Put the entry-point rule in one helper in a new module under
`al-vm/src/reports/`. Call it at the following boundaries before request-page
or dataset triggers:

| Entry point | Boundary |
|---|---|
| Report.Run and RunModal, including ProcessingOnly | `report_run` |
| Report export, including SaveAs | `report_save_core` |
| Report.WordXmlPart | `report_word_xml_part` |
| RunRequestPage and the request-page path from report execution | `report_request_only` and the shared request-page entry |

The IR changes `ReportExt::aliases`, `ReportExtAdd::item`,
`ReportExtModify::item` and adds `AddedItem::foreign_parent`. An item anchor
distinguishes base, local added and foreign items. A foreign reference carries
the owner extension object, its local item index and the resolved table
object. The symbolic linker and cache walker remap the owner and table. The
ordered report-extension vector carries execution order.

A consumer cache entry names its owner extension and table in its symbolic
dependencies. If an owner comes from cache, reuse its resolved item layout
with the remapped object identity. If its identity or layout has changed,
rebuild the consumer. Use the next free compile-cache format after #7912
because the serialized item anchors and aliases change shape.

Lowering records `dependency-report-data-items` via `ExtOrder::assumes` or
the shared report assumption plumbing when an actual foreign `modify`,
column `add`, placement, or explicit immediate-parent link resolves.
Visible candidates and aliases alone add no assumption. Use the existing
report assumption propagation. Issue #7878 supplies the reversed-ID
observation. Replace the temporary assumption with corpus file and line
citations once the pin includes both fixtures in corpus PR #1342.

## Not probed

Keep the refusal for a temporary owner item, another foreign `modify` trigger,
`modify` or `add` on a nested owner item, `addbefore` or `addafter` beside
the owner's top item, and child `addfirst` or `addlast` under a foreign item.
Also refuse an indirect or same-app owner, ambiguous names, a colliding
consumer variable, foreign references with three or more extensions, an
unrelated explicit link name, and foreign `RequestFilterFields`.

Other entry points remain refused, including `Report.SaveAsXml`, a report
variable's `SaveAs`, a `RecordRef` argument, non-empty request parameters,
the PDF, Word, Excel and HTML export formats, and the run and request-page
paths listed above.

## Consequences

A report with no foreign dataset references keeps its current runtime path and
assumption status. Build the name index once per report from the already
reached extension layouts, and consult it only for a foreign reference. This
keeps repeated consumer lookups inside the existing demand work budget. The
cache format moves once and old entries rebuild. Future extensions to placement
and renderer behavior need probes and updates in both the lowering and VM
entry-point guards.

## Conformance

Run both three-app regressions with static Report.SaveAs XML assertions
for trigger order, the dependent column value and sibling links. Replace the
ProcessingOnly RunModal positive regression with the XML regression and keep
RunModal as a refusal control. Cover cold compilation and cache/link
reconstruction, every listed refusal and the unused-candidate case. Run
`tools/demand_vs_eager.sh` and the baseline diff on all three versions.
Bring the implementation draft to this text after ADR acceptance.
