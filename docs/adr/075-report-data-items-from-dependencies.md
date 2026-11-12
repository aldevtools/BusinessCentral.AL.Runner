# ADR-075: Report data items from dependency extensions

- Status: Proposed
- Date: 2026-11-12

## Context

ADR-025 leaves references to data items introduced by another report extension unsupported. A neutral multi-app repro passes on BC 27.5, 28.5, and 29.0. A dependent extension modifies a data item, adds an XML dataset column to it, and places linked siblings around one of its children.

## Decision

Resolve unique data items in visible dependency extensions of the same report. Use the report's established extension order so a dependency's items are available when its consumer is lowered.

Carry the resulting report-extension rank through symbolic link joins so cached runs reconstruct the same dependency order.

A foreign item reference holds the owning extension object, its item index, and the table used for field linking. The IR walker remaps the owner and table through the linker and compile cache. Increase the cache format to 21.

The runtime binds the dependent extension's named record to the same buffer owned by the introducing extension. It uses the resolved item for trigger dispatch, dataset columns, and sibling placement. Keep the behavior flagged under the dependency-report-data-items assumption until the corpus pin covers it.

## Conformance

The local multi-app regression checks trigger order and explicit links to a parent introduced by a dependency. The neutral native repro additionally checks the dependent column's XML value on each supported BC version.
