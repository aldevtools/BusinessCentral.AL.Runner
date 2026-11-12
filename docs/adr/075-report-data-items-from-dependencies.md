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

Choose option 3. Resolve an unambiguous name from a direct dependency's
extension of the same report. Support the measured XML dataset operations:
`modify` with OnAfterAfterGetRecord, `add` columns, and sibling
`addbefore` / `addafter` placement. The extension-order owner in
`al-sema/src/lower/ext_order.rs` places the measured two-app consumer after
the owner even when their object IDs are reversed. Same-app and independent-app
ordering retain the ADR-025 rules.

A consumer's own declared name retains its existing meaning. Reject ambiguous
item names, items introduced by unrelated or indirect dependencies, temporary
items, unmeasured triggers and child placement, and `RequestFilterFields` in a
foreign-item modification. Reject an explicit `DataItemLinkReference` naming a
foreign item pending a dedicated pin. The existing implicit link used by the
measured sibling placement is covered here. Output formats other than XML
retain their current refusal when they reach a foreign item.

The IR uses an item anchor for a base, local added or foreign data item.
A foreign reference carries the owner extension object, that extension's local
item index and the resolved table object. The compile cache and symbolic linker
walk and remap the owner and table references. The local item index is relative
to its owner. The ordinary ordered report-extension vector carries execution
order. No new rank field or symbolic rank is serialized. Bump the dependency
cache format from 20 to 21 because the serialized anchors and report-extension
aliases change shape.

The VM binds a used alias to the owner's current row for dispatch and XML
output. Mark actual foreign references with the
`dependency-report-data-items` assumption until the new corpus pin reaches
the runner. An unused visible item must leave a clean result clean.

## Validation before merging the implementation

Run the reversed-ID three-app regression with assertions for trigger order,
XML column value and sibling links. Cover cold compilation and cache/link
reconstruction with the same fixture. Add refusal controls for ambiguous and
out-of-scope owners, foreign RequestFilterFields, explicit foreign parent links,
temporary items, and alternate renderers. Verify that a local declaration
keeps its name and that a report with an unused dependency item stays clean.
Run the three-major baseline diff. The implementation draft must implement
these boundaries before it is ready for code review.
