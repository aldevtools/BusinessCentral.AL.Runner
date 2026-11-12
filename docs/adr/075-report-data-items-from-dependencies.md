# ADR-075: Report data items from dependency extensions

- Status: Proposed
- Date: 2026-11-12
- Amends: [ADR-025](025-multiple-reportextensions-per-run.md)
- Related: [ADR-022](022-compile-cache.md)

## Context

A report extension can introduce a data item. A downstream extension in an app
that depends on its owner can refer to that item. The neutral repro in runner
issue #7878 passed on BC 27.5.46862.56996, 28.5.54151.57550 and
29.0.54011.57566. The consumer's report extension object ID is lower than the
owner's. The owner ran first, the consumer's OnAfterAfterGetRecord ran against
its row, and the serialized XML contained the consumer's column and the linked
siblings in their observed positions.

The proposed pin is
`reports/extensions/TestReportDependencyDataItems.al` in corpus PR #1342,
with the base, owner and consumer defined by
`al-language-xapp-a/ReportDependencyBase.Report.al`,
`al-language-xapp-b/ReportDependencyB.ReportExt.al` and
`al-language-xapp-c/ReportDependencyC.ReportExt.al`. The archived standalone
native run and the integrated corpus CI are separate evidence.

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

Support the measured XML dataset operations on a unique, directly dependent
owner item: `modify` with OnAfterAfterGetRecord, `add` columns, and sibling
`addbefore` / `addafter` placement. Use the existing placement and implicit
link rules. For an explicit `DataItemLinkReference`, allow only a name that
resolves to the foreign item that is the immediate parent. The corpus fixture's
`AfterFooter` names `AddedRow` and reads `after=2`. The enclosing-item rule
and its base and same-extension cases belong to runner PR #7912, which lands first.

Keep the established order rules in the module comment of
`al-sema/src/lower/ext_order.rs`. Permit the additional reversed-ID case only
when all these conditions hold:

1. Exactly two extensions change the report run, in distinct apps.
2. The consumer app declares the owner app as a direct dependency.
3. The consumer extension's object ID is lower than the owner's.
4. A consumer `modify` resolves to a unique item declared by that owner.
   A column or placement reference alone is outside the measured ordering rule.

The order module returns the owner followed by the consumer. Three or more
extensions stay on the current rules. A reversed pair without the qualifying
`modify` keeps the current cross-app order refusal.

A consumer's own declared variable retains its existing meaning. Leave the
foreign name unbound in that case and refuse if the report dataset attempts
the colliding foreign reference. Preserve the existing reason text for refused
shapes. Reject ambiguous items, unmeasured triggers and child placement, and
`RequestFilterFields` on a foreign modification during lowering.

The VM refuses unsupported entry points before request-page or dataset triggers:

| Entry point | Where the refusal belongs |
|---|---|
| Report.Run and RunModal, including ProcessingOnly | `al-vm/src/reports.rs`, `report_run` |
| SaveAs to PDF, Word, Excel or HTML | `al-vm/src/reports.rs`, `report_save_core`, before entering the report |
| Report.WordXmlPart | `al-vm/src/reports/word_xml_part.rs`, `report_word_xml_part` |
| RunRequestPage and the request-page path from report execution | `al-vm/src/reports.rs`, `report_request_only` and the shared request-page entry |

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

The VM marks `dependency-report-data-items` when it enters a supported XML
run that has a resolved foreign `modify`, column `add`, placement, or explicit
immediate-parent link. A visible candidate or alias without any such reference
leaves a clean run clean. Issue #7878 supplies the measured repro. Replace
the temporary assumption with corpus file and line citations once the pin
includes corpus PR #1342.

## Not probed

Keep the existing refusal for a temporary owner item, a different foreign
`modify` trigger, child `addfirst` or `addlast` under a foreign item, an indirect
or same-app owner, ambiguous item names, a colliding local name used as a
foreign item, reversed ordering with three or more extensions, an unrelated
explicit link name, foreign `RequestFilterFields`, and the other output and
request-page paths above.

## Consequences

A report with no foreign dataset references keeps its current runtime path and
assumption status. Build the name index once per report from the already
reached extension layouts, and consult it only for a foreign reference. This
keeps repeated consumer lookups inside the existing demand work budget. The
cache format moves once and old entries rebuild. Future extensions to placement
and renderer behavior need probes and updates in both the lowering and VM
entry-point guards.

## Conformance

Run the reversed-ID three-app regression with Report.SaveAs XML assertions
for trigger order, the dependent column value and sibling links. Replace the
ProcessingOnly RunModal positive regression with the XML regression and keep
RunModal as a refusal control. Cover cold compilation and cache/link
reconstruction, every listed refusal and the unused-candidate case. Run
`tools/demand_vs_eager.sh` and the baseline diff on all three versions.
Bring the implementation draft to this text after ADR acceptance.
