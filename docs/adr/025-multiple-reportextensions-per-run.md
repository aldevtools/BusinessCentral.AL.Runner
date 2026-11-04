# ADR-025: Several reportextensions that change one report run

- Status: Accepted (owner, 2026-10-14; build now)
- Date: 2026-10-14
- Changes: `ir::Report::extension` (one object) and `vm::Run::ext` (one instance) become lists; the cache facts `report_extension` (`al-sema/src/provided.rs`) becomes a list and `al_cache::FORMAT` rises. Additive in the IR (a new field, the old one kept); a change to a cache format. It needs the owner's decision because of CLAUDE.md rule 5 (seams) and ADR-022 (cache format).

## Context

The lowering refuses a report that more than one reportextension changes: "more than one reportextension changes the run (their order is not pinned)" (`al-sema/src/lower.rs`, `extension_reasons`). Microsoft's "Document Entries" has two such extensions in Base Application, "Mfg. Document Entries" (id 99000965) and "Serv. Document Entries" (id 6465). Page "Navigate" opens it, so every test that reaches `Navigate.OnAction` is refused statically.

Measured on BC 27.5 build 48827 (branch `adr-023`, 2026-10-14, pins on): 2828 page-side and about 400 report-side tests of the 31 buckets (SCM-Service left out) stop at this reason as their first stop. Four corpus tests (67402 and 68620, `PrecompiledPageExtControl_*`, `SourcePageExtModifyEditable_*`) pass in demand mode and are refused in eager mode through the same reason; real BC passes them.

The order is now pinned (BC 27.5, corpus `tests/al-language/reports/extensions/`):
- `tests/al-language/reports/extensions/TestReportExtensionBaseMembersContracts.al:256`: several reportextensions share the base's variables; for every trigger the base runs first, then the extensions in **object id order** (ids 68893 "Ext", 68894 "Zeta", 68895 "Alpha": Alpha runs last). Columns they add follow in the same order.
- `tests/al-language/reports/extensions/TestReportExtensionSamePlaceContracts.al:251`: items added at one place go next to the anchor, so the later extension is nearest: `addfirst(Parent)`: G1 F1 F2 (the later extension's items first, one extension's items in declaration order); `addbefore(ChildA)`: X1 X2 Y1 ChildA; `addafter(ChildA)`: ChildA Z2 Z1. Run order and dataset column order are the same.
- Not probed: order across apps; an anchor on an item another extension added; `addlast` of two extensions under one parent; two items added at one place by one extension with `addafter`.

## Decision (proposed)

1. **IR.** `Report::extensions: Vec<usize>` (`#[serde(default)]`) replaces `extension`: every reportextension that changes the run, in object id order (owner-review decision 1).
2. **Order.** Extensions sort by object id. Extensions of different apps (`ObjectSym::app`) are refused with their own reason ("reportextensions of different apps change the run (their order across apps is not pinned)"), also when an id is missing. The old reason stays for nothing else.
3. **Placement.** Each extension keeps its own `ReportExt::added`. At run time the items under a parent are woven by extension in id order: every `Before(a)` inserts immediately before the anchor and every `After(a)` immediately after it, so the later extension ends nearest; `First` items splice at the front per extension (the later one lands first); `Last` items append in id order.
4. **Refusals that stay.** An anchor on an item another extension added (already refused in `build_ext`); `addlast` under the same base parent from two extensions; two items at one place from one extension with `After` (the existing "two data items added at the same place" reason stays for that case only; `First` and `Before` in declaration order are pinned and can be allowed).
5. **VM.** `Run::ext: Option<(Inst, usize)>` becomes `Run::exts: Vec<(Inst, usize)>`; `ItemRef::Added(i)` becomes `Added(k, i)` with `k` the index in `exts`. Sites in `al-vm/src/reports.rs` (about 30):
   - `report_prepare` (line ~679, builds the instances and aliases), `report_run_for`, `Run::ext_ir`, `Run::item`, `Run::owner`, `Run::all_items`, `Run::children`;
   - the loop that runs `on_pre` / `on_post` of the extension (lines ~798 and ~816), the `modify` triggers `OnBeforeAfterGetRecord` / `OnAfterAfterGetRecord` (~872 to ~894), `dataset_autocalc` and `dataset_begin` (~1232 and ~1261, the `adds` columns), `apply_run_parameters` (~1211), the two `run_request_page` calls (~634, ~742) and `pages/request.rs` (`run_request_page`, line ~64, and the `SaveAsXml` path, line ~162, which read `report().extension`).
   Base trigger first, then each extension in order, for every trigger.
6. **Request pages.** `run_request_page`'s `run_ext: Option<(usize, Inst)>` becomes a list. The refusal "reportextension changes the run and extends the request page" stays per extension.
7. **Compile cache facts.** `ObjectFacts::report_extension: Option<u32>` (`provided.rs`) becomes `report_extensions: Vec<u32>`, and `lower.rs` (`Shared::report_extension`, line ~123 where a report's extension routines join the call graph, ~683 to ~703, ~892) reads all of them. Bump `al_cache::FORMAT` from 1 to 2 so existing entries are rejected and rebuilt (ADR-022 decision 4). A block that embeds the list of extensions already names the objects it depends on (ADR-022 decision 2); the extensions of a report join that list. A workspace reportextension of a cached dependency report forces that report to be lowered again, as for a page extension today.
8. **Demand and eager.** Demand reach already activates the extensions of an extended report; the change must not alter which routines are reached. The differential (`tools/demand_vs_eager.sh`, and the bucket comparison) must stay at zero differences.

## Consequences

- Document Entries lowers. Tests that stop only at this reason (about 2800 on the page side) run on to their next stop. The four corpus tests above no longer meet the refusal in eager mode; whether they then agree depends on what else the closure of their page chain meets (not shown).
- The cache entries of all users are rebuilt once (format 2).
- One more field in `Report`, one list in the cache facts, no removal.
- Cost when unused: none for a report with at most one extension (a one-element list).

## Decided without owner review (2026-10-14, by the coordinator)

1. `Report::extension` is replaced by `Report::extensions` (one source of truth); every reader is updated.
2. Reportextensions of different apps stay refused until probed.
3. `addlast` of two extensions under one parent and two items from one extension `After` stay refused until probed. `addfirst` and `addbefore` with several items of one extension are pinned (declaration order) and allowed.
4. `al_cache::FORMAT` is 3 (the zero-diff change took 2); the cache owner is no longer running, so `provided.rs` was reviewed by the implementer.

## Decisions that were needed

1. Accept the list design (new `extensions` field beside `extension`), or replace `extension` and update every reader.
2. Accept refusing extensions of different apps (order across apps unprobed), or ask for that probe first.
3. Accept refusing `addlast` of two extensions under one parent and `After` with two items from one extension, or ask for probes.
4. Bump `al_cache::FORMAT` to 2 in the same change (needed), and confirm the cache owner reviews the `provided.rs` part.

## Conformance

The two corpus files above, run as tests; VM tests in `crates/al-vm/tests/` with three extensions (ids not in name order) for trigger order, columns and the three placements; a lowering test for the two refusals; the cache test that a format-1 entry is rejected; `tools/demand_vs_eager.sh` clean; the Document Entries buckets re-measured with the recipe in `docs/alrunner-buckets.md`.
