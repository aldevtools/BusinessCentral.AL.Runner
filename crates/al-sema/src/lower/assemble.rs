//! Lowering a whole workspace: drives the objects (in parallel when it pays), builds the shared
//! data, and relowers the objects an edit touched.
use super::*;

/// The routines driven by the slots of a routine (its locals) and its object (its globals).
///
/// The edges are a function of the sources alone, never of how much of them was compiled
/// (owner decision of 2026-10-14: the demand-driven, the complete and the cached compile give one
/// outcome). The demand-driven compile reaches exactly these routines for the same slots
/// (`check/demand.rs`: `activate` for a page, report or xmlport that is used, `reach_run_objects`
/// for the `RunObject` of a workspace page), so that an edge never leads to a routine that was
/// not compiled.
pub(crate) fn driven_edges<'a>(
    d: &DrivenCtx,
    slots: impl Iterator<Item = &'a ir::Slot>,
    out: &mut Vec<(usize, usize)>,
) {
    let with_role = |q: usize, mask: u8, out: &mut Vec<(usize, usize)>| {
        out.extend(
            d.routine_roles[q]
                .iter()
                .enumerate()
                .filter(|(_, role)| **role & mask != 0)
                .map(|(ri, _)| (q, ri)),
        );
    };
    // The routines using object `q` reaches: all of a page's, the triggers of a report's.
    let used = |q: usize, out: &mut Vec<(usize, usize)>| {
        let mask = if d.triggers_only[q] {
            ROLE_TRIGGER | ROLE_SYNTHETIC
        } else {
            u8::MAX
        };
        out.extend(
            d.routine_roles[q]
                .iter()
                .enumerate()
                .filter(|(_, role)| mask == u8::MAX || **role & mask != 0)
                .map(|(ri, _)| (q, ri)),
        );
    };
    for s in slots {
        // `CurrPage` is the page's handle on itself, not a variable that drives it.
        if s.name == "CurrPage" {
            continue;
        }
        if let RtType::TestPage(p) | RtType::Page(p) = s.ty {
            let p = p as usize;
            // The demand-driven compile reaches the code of a `Page` variable's page when the
            // variable runs it (`Page.Run`), not when it is declared, so a dependency page
            // that a `Page` variable names adds no edge. A workspace page is always compiled.
            if matches!(s.ty, RtType::Page(_)) && d.dependency[p] {
                continue;
            }
            // The page and, transitively, the pages it shows as parts.
            let mut seen = vec![p];
            let mut at = 0;
            while at < seen.len() {
                let q = seen[at];
                at += 1;
                used(q, out);
                // The extensions' triggers and procedures run with the page.
                for &e in d.page_extensions.get(&q).map(|v| v.as_slice()).unwrap_or(&[]) {
                    used(e, out);
                }
                for &part in &d.page_parts[q] {
                    if !seen.contains(&part) {
                        seen.push(part);
                    }
                }
                // What the `RunObject` actions of workspace pages run: a page like a part, a
                // codeunit by its `OnRun`. Those of a dependency page are compiled (the test can
                // invoke the action, `Demand::reach_run_targets`) but add no edge: nothing says
                // that the test invokes it, and the code behind it refuses when it runs.
                if d.dependency[q] {
                    continue;
                }
                for &t in &d.page_runs[q] {
                    if d.codeunit[t] {
                        with_role(t, ROLE_ON_RUN, out);
                    } else if !seen.contains(&t) {
                        seen.push(t);
                    }
                }
            }
        }
        // An xmlport's triggers are run by Import and Export, and a report's by Run, which
        // the graph cannot see.
        if let RtType::Codeunit(p) = s.ty {
            let p = p as usize;
            if d.runs_triggers[p] {
                with_role(p, ROLE_TRIGGER | ROLE_SYNTHETIC, out);
            }
            for &e in &d.report_extension[p] {
                with_role(e, ROLE_TRIGGER | ROLE_SYNTHETIC, out);
            }
        }
    }
}

impl Lowered {
    /// The program, to change: copies it only if an engine already holds it.
    pub fn program_mut(&mut self) -> &mut ir::Program {
        std::sync::Arc::make_mut(&mut self.program)
    }
}

pub fn lower(ws: &Workspace, members: &MemberTable<'_>, checked: &CheckResult) -> Lowered {
    lower_with_positions(ws, members, checked, true)
}

/// `lower`, choosing whether statements carry source positions (`ir::Stmt::Pos`). Unit tests
/// that check the shape of lowered bodies turn them off.
pub fn lower_with_positions(
    ws: &Workspace,
    members: &MemberTable<'_>,
    checked: &CheckResult,
    positions: bool,
) -> Lowered {
    lower_with_workers(ws, members, checked, positions, lower_workers())
}

/// `lower_with_positions` with `workers` threads lowering the objects of dependency apps next to
/// the one that lowers the workspace's. The result does not depend on the number.
pub fn lower_with_workers(
    ws: &Workspace,
    members: &MemberTable<'_>,
    checked: &CheckResult,
    positions: bool,
    workers: usize,
) -> Lowered {
    lower_core(ws, members, checked, positions, workers, None, false).expect("no cache, no cache error")
}

/// What a compile cache supplies to a run (see `provided.rs`).
pub(crate) struct Provided<'a> {
    pub src: &'a dyn crate::provided::DepSource,
    /// The blocks of the objects that are supplied, read and checked.
    pub blocks: Vec<Option<crate::provided::DepBlock>>,
}

impl Provided<'_> {
    /// Whether object `oi` comes from the cache.
    pub(crate) fn has(&self, oi: usize) -> bool {
        self.blocks.get(oi).is_some_and(Option::is_some)
    }
}

/// The routine numbering of the objects: routine `r` of object `o` starts at `offsets[o][r]`.
/// Offsets ascend, so the number of a routine is found by search.
#[derive(Clone)]
pub struct RoutineAt(pub std::sync::Arc<Vec<Vec<usize>>>);

impl RoutineAt {
    pub fn get(&self, key: &(usize, usize)) -> Option<usize> {
        self.0.get(key.0)?.binary_search(&key.1).ok()
    }
}

/// What the lowering of an object reads, built from the workspace and the dependency facts, and the
/// subscribers of dependency objects that were not compiled.
pub(crate) struct Prologue<'a> {
    pub(crate) shared: Shared<'a>,
    pub(crate) uncompiled_subscribers: Vec<ir::UncompiledSubscriber>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_shared<'a>(
    ws: &'a Workspace,
    members: &'a MemberTable<'a>,
    checked: &'a CheckResult,
    catalog: &'a al_builtins::catalog::Catalog,
    positions: bool,
    src: Option<&'a dyn crate::provided::DepSource>,
    flags: &[bool],
    export: bool,
) -> Prologue<'a> {
    let prov = |oi: usize| flags[oi];
    // With dependency apps the checker found what the workspace reaches in them (ADR-013);
    // the rest is not compiled and no longer resolves.
    let demand = checked.demand.as_ref();
    if let Some(d) = demand {
        ws.restrict_to((0..ws.objects.len()).map(|i| d.active(i)).collect());
    }
    if let (Some(d), Some(path)) = (demand, std::env::var_os("AL_DUMP_REACH")) {
        // Debug aid: what the demand reached, one line per object (level) and routine.
        let mut lines: Vec<String> = Vec::new();
        for (i, o) in ws.objects.iter().enumerate() {
            if ws.is_dependency(i) && d.active(i) {
                lines.push(format!("{} {} level {}", o.kind.label(), o.name, d.level[i]));
            }
        }
        for &(i, off) in &d.reached {
            lines.push(format!(
                "{} {} routine {off}",
                ws.objects[i].kind.label(),
                ws.objects[i].name
            ));
        }
        lines.sort();
        let _ = std::fs::write(path, lines.join("\n"));
    }
    let active = |oi: usize| demand.is_none_or(|d| !ws.is_dependency(oi) || d.active(oi));
    let uncompiled_subscribers = if demand.is_some() {
        crate::check::demand::uncompiled_subscribers(ws, &active)
    } else {
        Vec::new()
    };
    // Routine numbering: every procedure and trigger node of an object, in source order. The
    // nodes of the objects this thread lowers stay in `nodes`; the start offset of every routine
    // is what other threads need, so it is asked of a node once.
    let active_v: Vec<bool> = (0..ws.objects.len()).map(active).collect();
    // Each thread builds the nodes of the objects it takes and keeps only the offsets.
    let (routine_offsets, routine_roles): (Vec<Vec<usize>>, Vec<Vec<u8>>) = {
        use rayon::prelude::*;
        let both: Vec<(Vec<usize>, Vec<u8>)> = (0..ws.objects.len())
            .into_par_iter()
            .map(|oi| {
                if prov(oi) {
                    let f = src.expect("provided").facts(oi);
                    return (
                        f.routine_offsets.iter().map(|&o| o as usize).collect(),
                        f.routine_roles.clone(),
                    );
                }
                if !active_v[oi] {
                    return (Vec::new(), Vec::new());
                }
                ast::routines_in(ws.object_ast(oi).syntax())
                    .iter()
                    .map(|r| (usize::from(r.syntax().text_range().start()), role_of(r)))
                    .unzip()
            })
            .collect();
        both.into_iter().unzip()
    };
    let routine_offsets = std::sync::Arc::new(routine_offsets);
    let routine_at = RoutineAt(routine_offsets.clone());
    // The table whose record is the implicit `Rec` of each object, and each table's extensions.
    let mut table_of: Vec<Option<usize>> = vec![None; ws.objects.len()];
    let mut extensions: HashMap<usize, Vec<usize>> = HashMap::default();
    // Pages: the control tree, the source table, and what the client cannot drive yet.
    let mut layouts: Vec<Option<crate::pages::Layout>> = (0..ws.objects.len()).map(|_| None).collect();
    let mut page_temporary: Vec<bool> = vec![false; ws.objects.len()];
    let mut page_extensions: HashMap<usize, Vec<usize>> = HashMap::default();
    for (oi, o) in ws.objects.iter().enumerate() {
        if prov(oi) {
            continue;
        }
        if o.kind == ObjKind::PageExtension && active(oi) {
            if let Some(base) = ws.object_ast(oi).extends_parts() {
                if let Resolution::Found(p) = ws.resolve_ref(o.file, ObjKind::Page, &base) {
                    page_extensions.entry(p).or_default().push(oi);
                }
            }
        }
        // A reportextension that extends the request page merges into it the way a page extension
        // merges into its page (corpus handlers/TestRequestPageControlErrorName.al).
        if o.kind == ObjKind::ReportExtension && active(oi) {
            let ext = ws.object_ast(oi);
            if ext.sections().any(|s| s.name().eq_ignore_ascii_case("requestpage")) {
                if let Some(base) = ext.extends_parts() {
                    if let Resolution::Found(r) = ws.resolve_ref(o.file, ObjKind::Report, &base) {
                        page_extensions.entry(r).or_default().push(oi);
                    }
                }
            }
        }
    }
    // Extensions apply dependencies first, then by object id: when two modify one property, the
    // later one wins (corpus handlers/TestPageExtensionModifyCaption_Tests.al: a higher object id wins
    // within an app, `Twice2Ctl`; the app that depends on the other wins whatever the ids, `FxBoth2Ctl`).
    for list in page_extensions.values_mut() {
        list.sort_by_key(|&e| (ws.app_depth(ws.objects[e].app), ws.objects[e].id, ws.objects[e].offset));
    }
    let mut page_reasons: Vec<Vec<String>> = vec![Vec::new(); ws.objects.len()];
    // The page objects each page shows as parts.
    let mut page_parts: Vec<Vec<usize>> = vec![Vec::new(); ws.objects.len()];
    // The pages and codeunits each page's RunObject actions run.
    let mut page_runs: Vec<Vec<usize>> = vec![Vec::new(); ws.objects.len()];
    // What the cache supplied about the objects it supplies.
    let usize_of = |v: &[u32]| -> Vec<usize> { v.iter().map(|&i| i as usize).collect() };
    if let Some(src) = src {
        for oi in (0..ws.objects.len()).filter(|&oi| prov(oi)) {
            let f = src.facts(oi);
            table_of[oi] = f.table_of.map(|t| t as usize);
            page_temporary[oi] = f.page_temporary;
            page_reasons[oi] = f.page_reasons.clone();
            page_parts[oi] = usize_of(&f.page_parts);
            page_runs[oi] = usize_of(&f.page_runs);
            if !f.page_extensions.is_empty() {
                page_extensions.insert(oi, usize_of(&f.page_extensions));
            }
        }
    }
    for (oi, o) in ws.objects.iter().enumerate() {
        if o.kind != ObjKind::Page || !active(oi) || prov(oi) {
            continue;
        }
        let obj = ws.object_ast(oi);
        let exts: Vec<(usize, ast::Object)> = page_extensions
            .get(&oi)
            .map(|v| v.iter().map(|&e| (e, ws.object_ast(e))).collect())
            .unwrap_or_default();
        let l = crate::pages::layout_with(&obj, &exts);
        page_reasons[oi].extend(l.reasons.iter().cloned());
        page_reasons[oi].extend(extension_reasons(&exts));
        for c in l.controls.iter().filter(|c| c.kind == ir::ControlKind::Part) {
            // A control an extension declares names its page the way the extension's file does.
            let file = ws.objects[c.owner.unwrap_or(oi)].file;
            let prop_of = |n: &str| {
                c.section
                    .properties()
                    .find(|p| p.name().eq_ignore_ascii_case(n))
                    .map(|p| p.value_text())
            };
            if let Some(link) = prop_of("SubPageLink") {
                if let Err(why) = crate::pages::parse_sub_page_link(&link) {
                    page_reasons[oi].push(format!("part {}: {why} is not supported yet", c.name));
                }
            }
            if let Some(provider) = prop_of("Provider") {
                let provider = provider.trim().trim_matches('"').to_string();
                if !l
                    .controls
                    .iter()
                    .any(|p| p.kind == ir::ControlKind::Part && p.name.eq_ignore_ascii_case(&provider))
                {
                    page_reasons[oi].push(format!(
                        "part {}: Provider `{provider}` is not a part of the page",
                        c.name
                    ));
                }
            }
            match crate::types::section_object_name(&c.section, 1) {
                Some(n) => match ws.resolve_ref(file, ObjKind::Page, &crate::types::qualified_parts(&n)) {
                    Resolution::Found(p) => page_parts[oi].push(p),
                    _ => page_reasons[oi].push(format!(
                        "part {} shows page {} outside the workspace",
                        c.name,
                        crate::types::display_name(&n)
                    )),
                },
                None => page_reasons[oi].push(format!("part {} names no page", c.name)),
            }
        }
        // The pages and codeunits the actions run: their routines are reached through the host
        // (a RunObject target runs when the action is invoked, which the call graph cannot see).
        for c in &l.controls {
            // How a `modify` block's RunObject would combine with the control's own is not pinned.
            if c.modifies
                .iter()
                .any(|(_, m)| m.properties().any(|p| p.name().eq_ignore_ascii_case("RunObject")))
            {
                page_reasons[oi].push(format!(
                    "modify({}) sets RunObject in a page extension (not supported yet)",
                    c.name
                ));
            }
            let Some(text) = c
                .section
                .properties()
                .find(|p| p.name().eq_ignore_ascii_case("RunObject"))
                .map(|p| p.value_text())
            else {
                continue;
            };
            if let Ok((kind, parts)) = crate::pages::parse_run_object(&text) {
                let found = match kind {
                    ir::RunKind::Page => Some(ObjKind::Page),
                    ir::RunKind::Codeunit => Some(ObjKind::Codeunit),
                    _ => None,
                };
                if let Some(Resolution::Found(t)) =
                    found.map(|k| ws.resolve_ref(ws.objects[c.owner.unwrap_or(oi)].file, k, &parts))
                {
                    if !page_runs[oi].contains(&t) {
                        page_runs[oi].push(t);
                    }
                }
            }
        }
        layouts[oi] = Some(l);
        let prop = |n: &str| {
            obj.properties()
                .find(|p| p.name().eq_ignore_ascii_case(n))
                .map(|p| p.value_text())
        };
        if let Some(t) = prop("SourceTable") {
            match ws.resolve_ref(o.file, ObjKind::Table, &[t.trim().trim_matches('"').to_string()]) {
                Resolution::Found(t) => table_of[oi] = Some(t),
                _ => page_reasons[oi].push(format!("source table: {}", ws.outside_workspace(ObjKind::Table, &t))),
            }
        }
        page_temporary[oi] = prop("SourceTableTemporary").is_some_and(|v| v.trim().eq_ignore_ascii_case("true"));
        // The values a SaveValues page stores are those of its variable-bound controls (corpus
        // handlers/TestPageSaveValues.al). An extension control bound to `Rec.Field` adds none, so
        // the stored set is the page's own. One bound to a variable is a control whose stored value
        // no corpus test pins (docs/probes-needed.md, "SaveValues with page extensions").
        if prop("SaveValues").is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
            && layouts[oi].iter().flat_map(|l| l.controls.iter()).any(|c| {
                c.owner.is_some()
                    && c.kind == ir::ControlKind::Field
                    && !c.section.name().eq_ignore_ascii_case("label")
                    && c.section.args().get(1).is_some_and(|src| {
                        !matches!(
                            crate::pages::classify_source(src, &std::collections::BTreeSet::new()),
                            crate::pages::Source::RecField(_)
                        ) && crate::pages::plain_name(src).is_some()
                    })
            })
        {
            page_reasons[oi].push(
                "SaveValues on a page that a page extension extends with a control bound to a variable: whether BC stores that variable is not pinned (docs/probes-needed.md)".into(),
            );
        }
    }
    // A page extension shares the page's `Rec`.
    for (&p, list) in &page_extensions {
        for &e in list {
            table_of[e] = table_of[p];
            page_temporary[e] = page_temporary[p];
        }
    }
    // A host cannot be driven where a part page cannot.
    for _ in 0..3 {
        for oi in 0..ws.objects.len() {
            for &p in &page_parts[oi].clone() {
                if let Some(why) = page_reasons[p].first().cloned() {
                    let reason = format!("part page `{}`: {why}", ws.objects[p].name);
                    if !page_reasons[oi].contains(&reason) {
                        page_reasons[oi].push(reason);
                    }
                }
            }
        }
    }
    for (oi, o) in ws.objects.iter().enumerate() {
        match o.kind {
            ObjKind::Table => table_of[oi] = Some(oi),
            _ if prov(oi) => {}
            // `Rec` of a codeunit with TableNo (a RunObject action hands it the host's record).
            ObjKind::Codeunit if active(oi) => {
                let table_no = ws
                    .object_ast(oi)
                    .properties()
                    .find(|p| p.name().eq_ignore_ascii_case("TableNo"))
                    .map(|p| p.value_text());
                if let Some(t) = table_no {
                    if let Resolution::Found(t) =
                        ws.resolve_ref(o.file, ObjKind::Table, &[t.trim().trim_matches('"').to_string()])
                    {
                        table_of[oi] = Some(t);
                    }
                }
            }
            ObjKind::TableExtension if active(oi) => {
                if let Some(base) = ws.object_ast(oi).extends_parts() {
                    if let Resolution::Found(t) = ws.resolve_ref(o.file, ObjKind::Table, &base) {
                        table_of[oi] = Some(t);
                        extensions.entry(t).or_default().push(oi);
                    }
                }
            }
            _ => {}
        }
    }
    // XmlPorts: schema and the reasons the runtime cannot run one yet. The element variables
    // take the first global slots (text, attribute and table nodes in schema order), then the
    // hidden source and destination streams, so the slots are known before anything is lowered.
    let mut xml_layouts: Vec<Option<crate::xmlports::Layout>> = (0..ws.objects.len()).map(|_| None).collect();
    let mut xml_irs: Vec<Option<ir::XmlPort>> = (0..ws.objects.len()).map(|_| None).collect();
    for (oi, o) in ws.objects.iter().enumerate() {
        if o.kind != ObjKind::XmlPort || !active(oi) || prov(oi) {
            continue;
        }
        let l = crate::xmlports::layout(&ws.object_ast(oi));
        let mut slots = HashMap::default();
        for n in &l.nodes {
            if matches!(
                n.kind,
                ir::XmlNodeKind::Text | ir::XmlNodeKind::TextAttr | ir::XmlNodeKind::Table
            ) {
                let next = slots.len();
                slots.entry(n.name.to_lowercase()).or_insert(next);
            }
        }
        let n = slots.len();
        let cx = crate::xmlports::Context {
            ws,
            members,
            oi,
            file: o.file,
            layout: &l,
            slots: &slots,
            routine_at: &routine_at,
            source_slot: n,
            destination_slot: n + 1,
        };
        xml_irs[oi] = Some(crate::xmlports::build(
            &cx,
            super::shared::over_max_length(&ws.object_ast(oi)),
        ));
        xml_layouts[oi] = Some(l);
    }
    // Reports: the dataset, and the reasons the runtime cannot run one yet. The data item records
    // take the first global slots (in source order), then the hidden `UseRequestPage` flag, so the
    // slots are known before anything is lowered. A reportextension that changes what a run does
    // (it has a procedure, a trigger or a section other than a request page) belongs to the
    // report it extends; one that changes nothing is ignored.
    let mut report_layouts: Vec<Option<crate::reports::Layout>> = (0..ws.objects.len()).map(|_| None).collect();
    let mut report_irs: Vec<Option<ir::Report>> = (0..ws.objects.len()).map(|_| None).collect();
    let mut ext_layouts: Vec<Option<crate::reports::ExtLayout>> = (0..ws.objects.len()).map(|_| None).collect();
    let mut ext_irs: Vec<Option<ir::ReportExt>> = (0..ws.objects.len()).map(|_| None).collect();
    let mut ext_base: Vec<Option<usize>> = vec![None; ws.objects.len()];
    // Per report: the reportextensions that change a run.
    let mut changing: HashMap<usize, Vec<usize>> = HashMap::default();
    for (oi, o) in ws.objects.iter().enumerate() {
        // Every reportextension of a report that is compiled counts, whether or not it was
        // compiled itself: what changes a run is decided by the installed apps' sources, never by
        // what a selection reached (issue 11). The demand-driven compile activates an extension with
        // its report, so the two agree today; this keeps them from drifting.
        if o.kind != ObjKind::ReportExtension || prov(oi) {
            continue;
        }
        if !active(oi) && !ws.extension_target(oi).is_some_and(&active) {
            continue;
        }
        if !ws.usable(oi) {
            continue;
        }
        let ext = ws.object_ast(oi);
        // The routines `pages::expand` adds for expression-valued control properties and sources of the
        // request page are not the author's: they do not change a run.
        let changes_a_run = ext.routines().any(|r| !crate::pages::is_synthetic(&r.name()))
            || ext.triggers().next().is_some()
            || ext.sections().any(|s| !s.name().eq_ignore_ascii_case("requestpage"));
        if let (true, Some(base)) = (changes_a_run, ext.extends_parts()) {
            if let Resolution::Found(r) = ws.resolve_ref(o.file, ObjKind::Report, &base) {
                changing.entry(r).or_default().push(oi);
                ext_base[oi] = Some(r);
                ext_layouts[oi] = Some(crate::reports::ext_layout(&ext));
            }
        }
    }
    for (oi, o) in ws.objects.iter().enumerate() {
        if o.kind != ObjKind::Report || !active(oi) || prov(oi) {
            continue;
        }
        let obj = ws.object_ast(oi);
        let l = crate::reports::layout(&obj);
        let mut slots = HashMap::default();
        for i in &l.items {
            let next = slots.len();
            slots.entry(i.name.to_lowercase()).or_insert(next);
        }
        // The request page: its controls and the data items a handler can filter. Why the test
        // client cannot drive it yet becomes a reason on the page, not on the report: a run
        // that never opens the page (UseRequestPage = false) does not need it.
        let exts: Vec<(usize, ast::Object)> = page_extensions
            .get(&oi)
            .map(|v| v.iter().map(|&e| (e, ws.object_ast(e))).collect())
            .unwrap_or_default();
        let (request, request_section) = crate::reports::request_layout(&obj, &l.items, &exts);
        page_reasons[oi].extend(request.reasons.iter().cloned());
        // A request page that declares `SourceTable` has a `Rec` of that table (corpus
        // handlers/TestReportRequestPageSourceTableCompiledContracts.al).
        if let Some(t) = request_section
            .as_ref()
            .and_then(|s| s.properties().find(|p| p.name().eq_ignore_ascii_case("SourceTable")))
            .map(|p| p.value_text())
        {
            match ws.resolve_ref(o.file, ObjKind::Table, &[t.trim().trim_matches('"').to_string()]) {
                Resolution::Found(t) => table_of[oi] = Some(t),
                _ => page_reasons[oi].push(format!("source table: {}", ws.outside_workspace(ObjKind::Table, &t))),
            }
        }
        // `SourceTableTemporary = true` gives the request page a temporary `Rec`: empty at OnOpenPage, and what it inserts stays out
        // of the table (report/lt-y-request-page/YRequestPageTests.al:99-113).
        page_temporary[oi] = request_section.as_ref().is_some_and(|s| {
            s.properties().any(|p| {
                p.name().eq_ignore_ascii_case("SourceTableTemporary")
                    && p.value_text().trim().eq_ignore_ascii_case("true")
            })
        });
        // Its variables would follow the base items' records in the extension's globals, which the
        // slots the request page binds to do not account for.
        for (e, _) in &exts {
            if changing.get(&oi).is_some_and(|c| c.contains(e)) {
                page_reasons[oi].push(format!(
                    "reportextension `{}` changes the run and extends the request page (not supported yet)",
                    ws.objects[*e].name
                ));
            }
        }
        layouts[oi] = Some(request);
        // Several extensions apply in object id order (corpus reportextensiontrigger/
        // TestReportExtensionBaseMembersContracts.al:256, ADR-025). Their order across apps is not
        // pinned.
        let mut exts = changing.get(&oi).cloned().unwrap_or_default();
        exts.sort_by_key(|&e| (ws.objects[e].id, e));
        let extension_reasons = if exts.len() > 1
            && (exts.iter().any(|&e| ws.objects[e].id.is_none())
                || exts.iter().any(|&e| ws.objects[e].app != ws.objects[exts[0]].app))
        {
            vec![
                "reportextensions of different apps change the run (their order across apps is not pinned)".to_string(),
            ]
        } else {
            Vec::new()
        };
        let cx = crate::reports::Context {
            ws,
            members,
            file: o.file,
            oi,
            layout: &l,
            slots: &slots,
            request_flag_slot: slots.len(),
            routine_at: &routine_at,
            extension_reasons,
            extensions: exts.clone(),
        };
        let mut over = super::shared::over_max_length(&obj);
        for &e in &exts {
            over.extend(super::shared::over_max_length(&ws.object_ast(e)));
        }
        report_irs[oi] = Some(crate::reports::build(&cx, &obj, over));
        report_layouts[oi] = Some(l);
    }
    for (ei, base) in ext_base.iter().enumerate() {
        let (Some(base), Some(layout)) = (*base, ext_layouts[ei].as_ref()) else {
            continue;
        };
        let (Some(base_layout), Some(base_ir)) = (report_layouts[base].as_ref(), report_irs[base].as_ref()) else {
            continue;
        };
        let cx = crate::reports::ExtContext {
            ws,
            members,
            file: ws.objects[ei].file,
            oi: ei,
            base,
            base_layout,
            base_items: &base_ir.items,
            layout,
            routine_at: &routine_at,
        };
        let (ir, reasons) = crate::reports::build_ext(&cx, &ws.object_ast(ei));
        ext_irs[ei] = Some(ir);
        // The extension's reasons stop the report it changes.
        let name = &ws.objects[ei].name;
        if let Some(r) = report_irs[base].as_mut() {
            r.unsupported.extend(
                reasons
                    .into_iter()
                    .map(|why| format!("reportextension \"{name}\": {why}")),
            );
        }
    }
    // Why a variable of an xmlport or report is refused, and the report extension that changes a
    // run: the cache supplies them for its objects.
    let n_objects = ws.objects.len();
    let mut xml_reasons: Vec<Option<String>> = (0..n_objects)
        .map(|oi| xml_irs[oi].as_ref().and_then(|p| p.unsupported.first().cloned()))
        .collect();
    let mut report_reasons: Vec<Option<String>> = (0..n_objects)
        .map(|oi| report_irs[oi].as_ref().and_then(|p| p.unsupported.first().cloned()))
        .collect();
    let mut report_extension: Vec<Vec<usize>> = (0..n_objects)
        .map(|oi| {
            report_irs[oi]
                .as_ref()
                .map(|r| r.extensions.clone())
                .unwrap_or_default()
        })
        .collect();
    if let Some(src) = src {
        for oi in (0..n_objects).filter(|&oi| prov(oi)) {
            let f = src.facts(oi);
            xml_reasons[oi] = f.xml_reason.clone();
            report_reasons[oi] = f.report_reason.clone();
            report_extension[oi] = f.report_extensions.iter().map(|&e| e as usize).collect();
        }
    }
    let driven = DrivenCtx {
        page_extensions,
        page_parts,
        page_runs,
        runs_triggers: ws
            .objects
            .iter()
            .map(|o| matches!(o.kind, ObjKind::XmlPort | ObjKind::Report))
            .collect(),
        report_extension,
        dependency: (0..n_objects).map(|oi| ws.is_dependency(oi)).collect(),
        codeunit: ws.objects.iter().map(|o| o.kind == ObjKind::Codeunit).collect(),
        triggers_only: ws
            .objects
            .iter()
            .map(|o| {
                matches!(
                    o.kind,
                    ObjKind::Report | ObjKind::ReportExtension | ObjKind::XmlPort | ObjKind::Query
                )
            })
            .collect(),
        routine_roles,
    };
    // From here on objects are lowered one at a time, each a function of the data above.
    let mut shared = Shared {
        ws,
        members,
        checked,
        catalog,
        demand,
        positions,
        early_relations: false,
        active: active_v,
        table_of,
        page_temporary,
        page_reasons,
        driven,
        xml_irs,
        report_irs,
        xml_reasons,
        report_reasons,
        ext_irs,
        ext_slots: Vec::new(),
        routine_offsets,
        routine_at,
        extensions,
        ext_base,
        export,
    };
    // The global slot of each variable of a page extension, by lower-case name. The page's control
    // tree is built before the extensions are lowered, and binds their controls to these slots.
    let mut ext_slot_table: Vec<HashMap<String, usize>> = vec![HashMap::default(); ws.objects.len()];
    {
        let local = Local::new(Nodes::new(ws, shared.active.clone()));
        let seq = Seq::new(&shared);
        for exts in shared.driven.page_extensions.values() {
            for &e in exts {
                if prov(e) {
                    continue;
                }
                let lw = make_lowerer(&shared, &local, &seq, e);
                let mut slots = HashMap::default();
                for v in ws.object_ast(e).var_decls() {
                    let ty = v.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown);
                    for n in v.names() {
                        if lw.rt(&ty).is_ok() {
                            let next = slots.len();
                            slots.insert(n.to_lowercase(), next);
                        }
                    }
                }
                ext_slot_table[e] = slots;
            }
        }
    }
    if let Some(src) = src {
        for e in (0..n_objects).filter(|&e| prov(e)) {
            ext_slot_table[e] = src
                .facts(e)
                .ext_slots
                .iter()
                .map(|(n, i)| (n.clone(), *i as usize))
                .collect();
        }
    }
    shared.ext_slots = ext_slot_table;
    Prologue {
        shared,
        uncompiled_subscribers,
    }
}

pub(crate) fn lower_core(
    ws: &Workspace,
    members: &MemberTable<'_>,
    checked: &CheckResult,
    positions: bool,
    workers: usize,
    provided: Option<Provided<'_>>,
    export: bool,
) -> Result<Lowered, String> {
    let catalog = al_builtins::catalog::Catalog::load();
    let (src, mut blocks): (
        Option<&dyn crate::provided::DepSource>,
        Vec<Option<crate::provided::DepBlock>>,
    ) = match provided {
        Some(p) => (Some(p.src), p.blocks),
        None => (None, Vec::new()),
    };
    let flags: Vec<bool> = (0..ws.objects.len())
        .map(|oi| blocks.get(oi).is_some_and(Option::is_some))
        .collect();
    let prov = |oi: usize| flags[oi];
    let Prologue {
        mut shared,
        uncompiled_subscribers,
    } = build_shared(ws, members, checked, &catalog, positions, src, &flags, export);
    let check = cfg!(test) || std::env::var_os("AL_CHECK_LINK").is_some();
    shared.early_relations = export || check;
    let tdd_sites = if checked.tdd {
        crate::check::tdd::sites_by_object(ws, &checked.resolutions)
    } else {
        Vec::new()
    };
    let mut program = ir::Program {
        objects: Vec::new(),
        apps: ws
            .apps
            .iter()
            .enumerate()
            .map(|(i, a)| ir::AppInfo {
                id: a.id.clone(),
                name: a.name.clone(),
                publisher: String::new(),
                version: String::new(),
                dependencies: a.dependencies.clone(),
                resources: Vec::new(),
                dependency: ws.kinds[i] == crate::workspace::AppKind::Dependency,
                include_source: false,
                target: None,
                layout_files: Vec::new(),
            })
            .collect(),
        dotnet_types: ws
            .dotnet
            .iter()
            .map(|d| ir::DotNetDecl {
                alias: d.decl.alias.clone(),
                assembly: d.decl.assembly.clone(),
                version: d.decl.version.clone(),
                culture: d.decl.culture.clone(),
                public_key_token: d.decl.public_key_token.clone(),
                type_name: d.decl.type_name.clone(),
            })
            .collect(),
        uncompiled_subscribers,
        tdd_stubs: tdd_stub_list(checked, &tdd_sites),
        ..Default::default()
    };
    let mut out = Lowered {
        timings: CompileTimings::default(),
        program: Default::default(),
        unsupported: Vec::new(),
        calls: Vec::new(),
        dependency: (0..ws.objects.len()).map(|oi| ws.is_dependency(oi)).collect(),
        provided: (0..ws.objects.len()).map(prov).collect(),
        driven: None,
        exported: None,
        al_diagnostics: checked
            .diagnostics
            .iter()
            .filter(|d| d.code.starts_with("AL"))
            .cloned()
            .collect(),
        pos_ranges: Vec::new(),
        checker_diags: checked.from_checker.clone(),
        skipped: checked.skipped.clone(),
        left_out: (0..ws.objects.len()).map(|oi| checked.is_left_out(oi)).collect(),
        tdd_sites,
        tdd_needs: if checked.tdd {
            crate::check::tdd_decl::needs(ws, checked)
        } else {
            Vec::new()
        },
    };
    if positions {
        program.sources = ws
            .files
            .iter()
            .map(|f| ir::SourceFile {
                app: f.app as u32,
                path: f.path.clone(),
                lines: Vec::new(),
            })
            .collect();
    }
    let n = ws.objects.len();
    // Worker threads and this one take the active objects in turn, the workspace's own first (they
    // are the biggest). Each object is lowered with its source positions numbered from 0; they
    // are moved to their place in the program's table below, in object order, so the numbering is
    // the one a single thread would have made.
    let mut theirs: Vec<usize> = (0..n)
        .filter(|&oi| shared.active[oi] && !ws.is_dependency(oi) && !prov(oi))
        .collect();
    theirs.extend((0..n).filter(|&oi| shared.active[oi] && ws.is_dependency(oi) && !prov(oi)));
    let workers = workers.min(theirs.len() / PARALLEL_OBJECTS_PER_WORKER);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let take_some = |local: &Local<'_>, seq: &Seq<'_>| -> Vec<(usize, Piece)> {
        let mut out = Vec::new();
        let mut line_cache: HashMap<usize, std::rc::Rc<FileLines>> = HashMap::default();
        loop {
            let k = next.fetch_add(LOWER_CHUNK, std::sync::atomic::Ordering::Relaxed);
            if k >= theirs.len() {
                return out;
            }
            for &oi in &theirs[k..(k + LOWER_CHUNK).min(theirs.len())] {
                out.push((oi, lower_object(&shared, local, seq, oi, &mut line_cache)));
            }
        }
    };
    let mut pieces: Vec<Option<Piece>> = (0..n).map(|_| None).collect();
    let local = Local::new(Nodes::new(ws, shared.active.clone()));
    let seq = Seq::new(&shared);
    let done: Vec<Vec<(usize, Piece)>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                std::thread::Builder::new()
                    .stack_size(LOWER_STACK)
                    .spawn_scoped(scope, || {
                        let local = Local::new(Nodes::new(ws, shared.active.clone()));
                        let seq = Seq::new(&shared);
                        take_some(&local, &seq)
                    })
                    .expect("spawn a lowering worker")
            })
            .collect();
        let mut done = vec![take_some(&local, &seq)];
        done.extend(
            handles
                .into_iter()
                .map(|h| h.join().expect("a lowering worker panicked")),
        );
        done
    });
    for (oi, piece) in done.into_iter().flatten() {
        pieces[oi] = Some(piece);
    }
    // Objects the cache supplies.
    {
        for (oi, slot) in blocks.iter_mut().enumerate() {
            if let Some(b) = slot.take() {
                pieces[oi] = Some(Piece {
                    object: ir::Object::default(),
                    sym: b.object,
                    positions: Vec::new(),
                    unsupported: b.unsupported,
                    calls: b
                        .calls
                        .iter()
                        .map(|r| r.iter().map(|&(o, r)| (o as usize, r as usize)).collect())
                        .collect(),
                    direct: Vec::new(),
                    embeds: Vec::new(),
                    field_refs: Vec::new(),
                    control_refs: Vec::new(),
                    expansions: Vec::new(),
                    page_mods: Default::default(),
                    key_owners: Vec::new(),
                    implements: src.map_or_else(Vec::new, |s| s.facts(oi).implements.clone()),
                    enum_impls: src.map_or_else(Vec::new, |s| s.facts(oi).enum_impls.clone()),
                    enum_ordinals: src.map_or_else(Vec::new, |s| s.facts(oi).enum_ordinals.clone()),
                    fields: src.map_or_else(Vec::new, |s| s.facts(oi).fields.clone()),
                    controls: src.map_or_else(Vec::new, |s| s.facts(oi).controls.clone()),
                    key: src.map(|s| s.facts(oi).key.clone()),
                });
            }
        }
    }
    // Objects nothing compiled are shells.
    let mut line_cache: HashMap<usize, std::rc::Rc<FileLines>> = HashMap::default();
    #[allow(clippy::needless_range_loop)] // `oi` is the object, not just a slot of `pieces`
    for oi in 0..n {
        if pieces[oi].is_none() {
            pieces[oi] = Some(lower_object(&shared, &local, &seq, oi, &mut line_cache));
        }
    }
    let routine_keys = super::localize::RoutineKeys::new(
        ws,
        (0..n)
            .map(|oi| src.filter(|_| prov(oi)).map(|s| s.facts(oi).routine_keys.as_slice()))
            .collect(),
        shared.active.clone(),
    );
    let implements = |cu: usize| implemented_interfaces(ws, cu);
    let enum_info = |i: usize| enum_info_of(ws, members, i);
    let piece_keys: Vec<al_ir::sym::ObjKey> = pieces
        .iter()
        .map(|p| {
            let p = p.as_ref().expect("every object is lowered");
            p.key
                .clone()
                .unwrap_or_else(|| al_ir::sym::ObjKey::of(&p.object, &program.apps))
        })
        .collect();
    // The subscribers of objects nothing compiled, per subscribing object (ADR-033).
    let mut subscriber_decls: Vec<Vec<al_ir::sym::SubscriberDecl>> = vec![Vec::new(); n];
    if shared.demand.is_some() {
        for site in crate::check::demand::uncompiled_subscriber_sites(ws, &|oi| shared.active[oi]) {
            let o = &ws.objects[site.object];
            let sub = &o.subscribers[site.index];
            subscriber_decls[site.object].push(al_ir::sym::SubscriberDecl {
                target: match (site.publisher, site.platform) {
                    (Some(p), _) => al_ir::sym::SubscriberTarget::Object(piece_keys[p].clone()),
                    (None, ir::PlatformPublisher::CaptionClass) => al_ir::sym::SubscriberTarget::CaptionClass,
                    (None, ir::PlatformPublisher::ReportingTriggers) => al_ir::sym::SubscriberTarget::ReportingTriggers,
                    (None, ir::PlatformPublisher::FilterHelperTriggers) => {
                        al_ir::sym::SubscriberTarget::FilterHelperTriggers
                    }
                    (None, _) => al_ir::sym::SubscriberTarget::GlobalTriggers,
                },
                index: site.index as u32,
                event: sub.event.to_lowercase(),
                element: sub.element.clone(),
                subscriber: format!("{} \"{}\"", o.kind.label(), o.name),
            });
        }
    }
    let enum_facts = |oi: usize| -> (Vec<al_ir::sym::EnumImplDecl>, Vec<u32>) {
        if !matches!(ws.objects[oi].kind, ObjKind::Enum | ObjKind::EnumExtension) {
            return (Vec::new(), Vec::new());
        }
        let (decls, ordinals) = enum_info_with(ws, members, oi, ws.objects[oi].kind == ObjKind::Enum);
        let key = |i: Option<usize>| i.map(|i| piece_keys[i].clone());
        (
            decls
                .into_iter()
                .map(|(slot, interface, codeunit)| al_ir::sym::EnumImplDecl {
                    slot,
                    interface: key(interface),
                    codeunit: key(codeunit),
                })
                .collect(),
            ordinals,
        )
    };
    let mut exported = export.then(crate::provided::DepSet::default);
    let mut staged: Vec<Staged> = Vec::with_capacity(n);
    let mut block_meta: Vec<BlockMeta> = Vec::new();
    let embeds_of_all: Vec<Vec<usize>> = pieces
        .iter()
        .map(|p| p.as_ref().map_or_else(Vec::new, |p| p.embeds.clone()))
        .collect();
    for (oi, piece) in pieces.into_iter().flatten().enumerate() {
        if let Some(set) = exported.as_mut() {
            // The block itself is made from the symbolic object, after the objects are made symbolic.
            block_meta.push((
                piece.unsupported.clone(),
                piece
                    .direct
                    .iter()
                    .map(|r| r.iter().map(|&(o, r)| (o as u32, r as u32)).collect())
                    .collect(),
            ));
            let o = &ws.objects[oi];
            let recorded: std::collections::BTreeSet<usize> = embeds_of_all[oi].iter().copied().collect();
            set.facts.push(crate::provided::DepFacts {
                key: al_ir::sym::ObjKey::of(&piece.object, &program.apps),
                kind: Some(o.kind),
                id: o.id,
                name: o.name.clone(),
                app: o.app as u32,
                reads: crate::provided::embeds_of(oi, &recorded, &|p| shared.driven.page_parts[p].clone())
                    .into_iter()
                    .map(|i| piece_keys[i as usize].clone())
                    .collect(),
                table_of: shared.table_of[oi].map(|t| t as u32),
                page_temporary: shared.page_temporary[oi],
                page_reasons: shared.page_reasons[oi].clone(),
                page_parts: shared.driven.page_parts[oi].iter().map(|&p| p as u32).collect(),
                page_runs: shared.driven.page_runs[oi].iter().map(|&p| p as u32).collect(),
                page_extensions: shared
                    .driven
                    .page_extensions
                    .get(&oi)
                    .map(|v| v.iter().map(|&p| p as u32).collect())
                    .unwrap_or_default(),
                ext_slots: {
                    let mut v: Vec<(String, u32)> = shared.ext_slots[oi]
                        .iter()
                        .map(|(n, &i)| (n.clone(), i as u32))
                        .collect();
                    v.sort();
                    v
                },
                routine_offsets: shared.routine_offsets[oi].iter().map(|&o| o as u32).collect(),
                routine_roles: shared.driven.routine_roles[oi].clone(),
                fields: piece
                    .object
                    .table
                    .as_ref()
                    .map_or_else(Vec::new, |t| t.fields.iter().map(|f| f.id).collect()),
                controls: piece.object.page.as_ref().map_or_else(Vec::new, |p| {
                    p.controls.iter().map(|c| c.name.to_ascii_lowercase()).collect()
                }),
                enum_impls: enum_facts(oi).0,
                enum_ordinals: enum_facts(oi).1,
                implements: implements(oi).into_iter().map(|i| piece_keys[i].clone()).collect(),
                routine_keys: routine_keys.of(oi).to_vec(),
                xml_reason: shared.xml_reasons[oi].clone(),
                report_reason: shared.report_reasons[oi].clone(),
                report_extensions: shared.driven.report_extension[oi].iter().map(|&e| e as u32).collect(),
                query: checked.queries[oi].clone(),
            });
        }
        out.unsupported.push(piece.unsupported);
        out.calls.push(piece.calls);
        staged.push(Staged {
            object: piece.object,
            positions: piece.positions,
            key: piece.key,
            field_refs: piece.field_refs,
            control_refs: piece.control_refs,
            expansions: piece.expansions,
            page_mods: piece.page_mods,
            sym: piece.sym,
            page_contribution: None,
            implements: piece.implements,
            enum_impls: piece.enum_impls,
            enum_ordinals: piece.enum_ordinals,
            key_owners: piece.key_owners,
            table_contribution: None,
            fields: piece.fields,
            controls: piece.controls,
        });
    }
    let field_pos = |t: usize, id: u32| members.get(t).fields.iter().position(|f| f.id == id);
    let field_no = |t: usize, pos: usize| members.get(t).fields.get(pos).map(|f| f.id);
    let field_numbers = |t: usize| members.get(t).fields.iter().map(|f| f.id).collect::<Vec<u32>>();
    let extends = |e: usize| ws.extension_target(e);
    let join_order = |e: usize, list: al_ir::sym::ListKind| {
        let o = &ws.objects[e];
        let id = o.id.map_or(0, |x| x.saturating_add(1));
        match list {
            al_ir::sym::ListKind::TableExtensions => (0, 0, 0),
            al_ir::sym::ListKind::PageExtensions => (ws.app_depth(o.app) as u32, id, o.offset as u32),
            al_ir::sym::ListKind::ReportExtensions => (id, 0, 0),
        }
    };
    let own_field_properties = |i: usize| {
        members
            .own(i)
            .fields
            .into_iter()
            .map(|f| f.properties)
            .collect::<Vec<_>>()
    };
    let over_max_length = |e: usize| {
        if ws.usable(e) {
            super::shared::over_max_length(&ws.object_ast(e))
        } else {
            Vec::new()
        }
    };
    let modify_properties = |e: usize| {
        if ws.usable(e) && ws.objects[e].kind == ObjKind::TableExtension {
            crate::members::modify_properties(ws, e)
        } else {
            Vec::new()
        }
    };
    let own_enum = |i: usize| {
        let m = members.own(i);
        (
            m.enum_values.iter().map(|(v, n)| (*v as i32, n.clone())).collect(),
            m.enum_captions.iter().map(|(v, c)| (*v as i32, c.clone())).collect(),
        )
    };
    let report_item_table = |r: usize, item: usize| {
        shared
            .report_irs
            .get(r)
            .and_then(|x| x.as_ref())
            .and_then(|x| x.items.get(item))
            .and_then(|i| i.table)
    };
    let lookups = Lookups {
        table_of: &shared.table_of,
        field_no: &field_no,
        field_pos: &field_pos,
        field_numbers: &field_numbers,
        report_item_table: &report_item_table,
        extends: &extends,
        join_order: &join_order,
        own_enum: &own_enum,
        implements: &implements,
        enum_info: &enum_info,
        own_field_properties: &own_field_properties,
        modify_properties: &modify_properties,
        over_max_length: &over_max_length,
    };
    let Linking { linked, expected, syms } = link_pieces(
        staged,
        program,
        &prov,
        &routine_keys,
        &lookups,
        subscriber_decls,
        LinkOptions { check, export },
    )?;
    if let Some(set) = exported.as_mut() {
        for ((unsupported, calls), object) in block_meta.into_iter().zip(syms) {
            set.blocks.push(crate::provided::DepBlock {
                object,
                unsupported,
                calls,
            });
        }
    }
    out.pos_ranges = linked.pos_ranges;
    let mut program = linked.program;
    if !shared.early_relations || check {
        derive_relations(&mut program, &shared, &prov, lower_workers());
    }
    if let Some((p, ranges)) = expected {
        if let Some(d) = al_link::first_difference(&p, &program) {
            return Err(format!(
                "AL_CHECK_LINK: the linked program differs from the lowered one: {d}"
            ));
        }
        if ranges != out.pos_ranges {
            return Err("AL_CHECK_LINK: the source position ranges differ".into());
        }
    }
    out.exported = exported;
    if src.is_some() {
        out.driven = Some(shared.driven.clone());
    }
    out.program = std::sync::Arc::new(program);
    Ok(out)
}

/// Works out what the `TableRelation` of each field gives the runtime, from the properties the table and its
/// extensions leave (a `modify` block can change `TableRelation`): the arms, the arms of a relation with filter
/// conditions, and the metadata of a relation the runtime does not understand.
fn derive_relations(
    program: &mut ir::Program,
    shared: &Shared<'_>,
    provided: &(dyn Fn(usize) -> bool + Sync),
    workers: usize,
) {
    type Relations = Vec<(
        Vec<ir::RelationArm>,
        Vec<ir::RelationArm>,
        Vec<ir::MetaRelationArm>,
        Vec<String>,
    )>;
    let ws = shared.ws;
    let tables: Vec<usize> = (0..program.objects.len())
        .filter(|&oi| !provided(oi) && !program.objects[oi].uncompiled && program.objects[oi].table.is_some())
        .collect();
    // The syntax cells of a worker are made once for a run of tables, not for each table.
    let runs: Vec<Vec<usize>> = tables.chunks(16).map(<[usize]>::to_vec).collect();
    let done: Vec<(usize, Relations)> = al_link::par_map(runs, workers, |_, run| {
        let local = Local::new(Nodes::new(ws, shared.active.clone()));
        let seq = Seq::new(shared);
        run.into_iter()
            .map(|oi| {
                let lw = make_lowerer(shared, &local, &seq, oi);
                let relations = shared.members[oi]
                    .fields
                    .iter()
                    .map(|f| {
                        let texts: Vec<String> = f
                            .properties
                            .iter()
                            .filter(|(k, _)| k.eq_ignore_ascii_case("TableRelation"))
                            .map(|(_, v)| v.clone())
                            .collect();
                        let (arms, runtime, meta) = lw.field_relations(oi, f);
                        (arms, runtime, meta, if texts.len() > 1 { texts } else { Vec::new() })
                    })
                    .collect();
                (oi, relations)
            })
            .collect::<Vec<_>>()
    })
    .into_iter()
    .flatten()
    .collect();
    for (oi, relations) in done {
        let Some(table) = program.objects[oi].table.as_mut() else {
            continue;
        };
        for (f, (arms, runtime, meta, texts)) in table.fields.iter_mut().zip(relations) {
            f.relation_arms = arms;
            f.relation_runtime = runtime;
            f.relation_meta = meta;
            f.relation_texts = texts;
        }
    }
}

/// What the lowering knew about tables, which making an object symbolic reads again.
struct Lookups<'a> {
    table_of: &'a [Option<usize>],
    field_no: &'a (dyn Fn(usize, usize) -> Option<u32> + Sync),
    /// The position of field `id` in table `table`.
    field_pos: &'a (dyn Fn(usize, u32) -> Option<usize> + Sync),
    /// The number of every field of table `table`, in order.
    field_numbers: &'a (dyn Fn(usize) -> Vec<u32> + Sync),
    report_item_table: &'a (dyn Fn(usize, usize) -> Option<usize> + Sync),
    extends: &'a (dyn Fn(usize) -> Option<usize> + Sync),
    join_order: &'a (dyn Fn(usize, al_ir::sym::ListKind) -> (u32, u32, u32) + Sync),
    own_enum: &'a (dyn Fn(usize) -> super::localize::EnumOwn + Sync),
    implements: &'a (dyn Fn(usize) -> Vec<usize> + Sync),
    enum_info: &'a (dyn Fn(usize) -> super::localize::EnumInfo + Sync),
    /// The properties of the fields a table declares itself, in order.
    own_field_properties: &'a (dyn Fn(usize) -> Vec<Vec<(String, String)>> + Sync),
    /// The `modify(Field)` blocks of a table extension: field name and the properties it sets.
    modify_properties: &'a (dyn Fn(usize) -> super::localize::Modifies + Sync),
    /// Why a table extension's properties refuse its table (see [`super::shared::over_max_length`]).
    over_max_length: &'a (dyn Fn(usize) -> Vec<String> + Sync),
}

/// For the check: turns the references the expressions of an object hold into the positions
/// lowering used to write there.
struct Unembed<'a> {
    expansions: &'a [super::localize::Expanded],
    list: &'a [(usize, u32)],
    controls: &'a [(usize, String, usize)],
    kinds: &'a [ObjKind],
    field_pos: &'a dyn Fn(usize, u32) -> Option<usize>,
}

impl al_ir::walk::RefVisitor for Unembed<'_> {
    fn object(&mut self, _: &mut usize) {}
    fn routine(&mut self, _: &mut usize, _: usize) {}
    fn expansion(&mut self, index: usize, site: al_ir::walk::ExpansionSite<'_>) {
        self.expansions[index].fill(site);
    }
    fn control(&mut self, r: &mut usize) {
        *r = self.controls[*r].2;
    }
    fn field(&mut self, r: &mut usize, table: al_ir::walk::TableOf) {
        if table != al_ir::walk::TableOf::Embedded {
            return;
        }
        let (t, id) = self.list[*r];
        // A query's column is named by its position, which is the number.
        *r = if self.kinds[t] == ObjKind::Query {
            id as usize
        } else {
            (self.field_pos)(t, id).unwrap_or_else(|| {
                panic!(
                    "field {id} of object {t} (kind {}), which the lowering named, is not in its members",
                    self.kinds[t]
                )
            })
        };
    }
}

/// The `OnBeforeValidate` and `OnAfterValidate` triggers one extension attaches to the field of a name: (extension, field name, before, after).
type ExtTriggers = (usize, String, Vec<(usize, usize)>, Vec<(usize, usize)>);

/// The object-level properties of a table that a table extension sets on it: the extension's value is the table's
/// (record/lt-x-table-props/XTablePropsTests.al:TableProperties_Metadata_LookupDrillDownDataCaption :585-587,
/// TableExt_DataCaptionFields_TestPageCaption :609).
const TABLE_EXTENSION_TABLE_PROPERTIES: &[&str] = &["DataCaptionFields", "LookupPageId", "DrillDownPageId"];
use al_ir::sym::UNSUPPORTED_TABLE_PROPERTY;

/// Takes what the table extensions declare out of the tables that hold it: their fields, their keys
/// and their changes to other fields (`modify`). The table keeps its own; link merges the rest again.
fn split_tables(
    staged: &mut [Staged],
    provided: &(dyn Fn(usize) -> bool + Sync),
    lookups: &Lookups<'_>,
    keep_relations: bool,
) {
    use al_ir::sym::{FieldModify, KeyDecl, TableContribution};
    let n = staged.len();
    let mut contributions: Vec<Option<TableContribution>> = vec![None; n];
    // The table extensions of each table, in program order.
    let mut exts_of: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::default();
    #[allow(clippy::needless_range_loop)] // `e` is the object, which `extends` takes
    for e in 0..n {
        if staged[e].object.kind == ObjKind::TableExtension {
            if let Some(b) = (lookups.extends)(e) {
                exts_of.entry(b).or_default().push(e);
            }
        }
    }
    #[allow(clippy::needless_range_loop)] // `b` is the object, which `provided` and the lookups take
    for b in 0..n {
        if provided(b) || staged[b].object.uncompiled || staged[b].object.table.is_none() {
            continue;
        }
        let owners = std::mem::take(&mut staged[b].key_owners);
        let mut table = staged[b].object.table.take().expect("checked");
        // The properties the table declares itself differ from the merged ones only where an extension has a
        // `modify` block for a field of the table.
        let modified = exts_of
            .get(&b)
            .is_some_and(|es| es.iter().any(|&e| !(lookups.modify_properties)(e).is_empty()));
        let own_props = if modified {
            (lookups.own_field_properties)(b)
        } else {
            Vec::new()
        };
        // Fields: the table's own and system fields stay.
        let all = std::mem::take(&mut table.fields);
        let mut kept = Vec::new();
        let mut by_ext: Vec<(usize, Vec<ir::Field>)> = Vec::new();
        // Changes the extensions make to fields: the triggers they attach, by extension and field name.
        let mut triggers: Vec<ExtTriggers> = Vec::new();
        for mut f in all {
            // What the relation of a field gives the runtime is worked out after link, from the properties
            // the extensions leave (`derive_relations`).
            // (A cache block keeps them: its set has the table and its extensions, so they are right for it.)
            if !keep_relations {
                f.relation_arms.clear();
                f.relation_runtime.clear();
                f.relation_meta.clear();
            }
            for (is_before, list) in [
                (true, std::mem::take(&mut f.ext_before_validate)),
                (false, std::mem::take(&mut f.ext_after_validate)),
            ] {
                for (e, r) in list {
                    let slot = match triggers
                        .iter()
                        .position(|(x, name, ..)| *x == e && name.eq_ignore_ascii_case(&f.name))
                    {
                        Some(i) => i,
                        None => {
                            triggers.push((e, f.name.clone(), Vec::new(), Vec::new()));
                            triggers.len() - 1
                        }
                    };
                    // Fields of one name carry the same triggers: take them once.
                    let (_, _, before, after) = &mut triggers[slot];
                    let target = if is_before { before } else { after };
                    if !target.contains(&(e, r)) {
                        target.push((e, r));
                    }
                }
            }
            match f.declared_in {
                None => kept.push(f),
                Some(d) if d as usize == b => kept.push(f),
                Some(d) => match by_ext.iter_mut().find(|(e, _)| *e == d as usize) {
                    Some((_, v)) => v.push(f),
                    None => by_ext.push((d as usize, vec![f])),
                },
            }
        }
        if modified && own_props.len() == kept.len() {
            for (f, props) in kept.iter_mut().zip(own_props) {
                f.properties = props;
            }
        }
        table.fields = kept;
        // Keys.
        let mut base_keys = (Vec::new(), Vec::new(), Vec::new());
        let mut ext_keys: Vec<(usize, KeyDecl)> = Vec::new();
        for (i, k) in table.keys.drain(..).enumerate() {
            let unique = table.unique_keys[i];
            let props = std::mem::take(&mut table.key_props[i]);
            match owners.get(i).copied() {
                Some(o) if o != b => ext_keys.push((
                    o,
                    KeyDecl {
                        fields: k,
                        unique,
                        props,
                    },
                )),
                _ => {
                    base_keys.0.push(k);
                    base_keys.1.push(unique);
                    base_keys.2.push(props);
                }
            }
        }
        (table.keys, table.unique_keys, table.key_props) = base_keys;
        // Hand it to the extensions.
        let mut touched: Vec<usize> = by_ext.iter().map(|(e, _)| *e).collect();
        touched.extend(ext_keys.iter().map(|(e, _)| *e));
        touched.extend(triggers.iter().map(|(e, ..)| *e));
        touched.extend(exts_of.get(&b).into_iter().flatten().copied());
        touched.sort_unstable();
        touched.dedup();
        for e in touched {
            // The table properties an extension may set (record/lt-x-table-props/XTablePropsTests.al:585-587, :609).
            let ext_props: Vec<(String, String)> = staged[e]
                .object
                .properties
                .iter()
                .filter(|(k, _)| {
                    TABLE_EXTENSION_TABLE_PROPERTIES
                        .iter()
                        .any(|n| n.eq_ignore_ascii_case(k))
                })
                .cloned()
                .chain(
                    (lookups.over_max_length)(e)
                        .into_iter()
                        .map(|why| (UNSUPPORTED_TABLE_PROPERTY.to_string(), why)),
                )
                .collect();
            let c = contributions[e].get_or_insert_with(|| {
                let modifies = (lookups.modify_properties)(e)
                    .into_iter()
                    .map(|(field, properties)| FieldModify {
                        field,
                        properties,
                        before_validate: Vec::new(),
                        after_validate: Vec::new(),
                    })
                    .collect();
                TableContribution {
                    fields: Vec::new(),
                    keys: Vec::new(),
                    modifies,
                    properties: ext_props,
                }
            });
            if let Some(pos) = by_ext.iter().position(|(x, _)| *x == e) {
                c.fields.extend(by_ext.swap_remove(pos).1);
            }
            c.keys
                .extend(ext_keys.iter().filter(|(x, _)| *x == e).map(|(_, k)| k.clone()));
            for (_, name, before, after) in triggers.iter().filter(|(x, ..)| *x == e) {
                if let Some(m) = c.modifies.iter_mut().find(|m| m.field.eq_ignore_ascii_case(name)) {
                    m.before_validate.extend(before.iter().copied());
                    m.after_validate.extend(after.iter().copied());
                }
            }
        }
        staged[b].object.table = Some(table);
    }
    for (e, c) in contributions.into_iter().enumerate() {
        staged[e].table_contribution = c;
    }
}

/// What enum (or enum extension) `i` declares itself: the implementations with their names resolved, and the
/// ordinals of its values.
fn enum_info_of(ws: &Workspace, members: &MemberTable<'_>, i: usize) -> super::localize::EnumInfo {
    enum_info_with(ws, members, i, false)
}

/// [`enum_info_of`], with the declarations and values of the extensions when `merged` (what a cache holds for an
/// enum: the extensions of its own set are part of it).
fn enum_info_with(ws: &Workspace, members: &MemberTable<'_>, i: usize, merged: bool) -> super::localize::EnumInfo {
    use crate::members::ImplSlot as M;
    use al_ir::sym::ImplSlot as S;
    let owned;
    let m = if merged {
        members.get(i)
    } else {
        owned = members.own(i);
        &owned
    };
    let decls = m
        .enum_impls
        .iter()
        .map(|im| {
            let file = ws.objects[im.declared_in].file;
            let found = |kind, parts: &[String]| match ws.resolve_ref(file, kind, parts) {
                Resolution::Found(x) => Some(x),
                _ => None,
            };
            (
                match im.slot {
                    M::Value(o) => S::Value(o),
                    M::Default => S::Default,
                    M::Unknown => S::Unknown,
                },
                found(ObjKind::Interface, &im.interface),
                found(ObjKind::Codeunit, &im.codeunit),
            )
        })
        .collect();
    (decls, m.enum_values.iter().map(|(v, _)| *v).collect())
}

/// The interfaces codeunit `cu` implements: the ones it lists and the ones those extend, in workspace numbering.
fn implemented_interfaces(ws: &Workspace, cu: usize) -> Vec<usize> {
    let mut out = Vec::new();
    if ws.objects[cu].kind != ObjKind::Codeunit {
        return out;
    }
    let file = ws.objects[cu].file;
    for p in &ws.objects[cu].implements {
        if let Resolution::Found(j) = ws.resolve_ref(file, ObjKind::Interface, p) {
            for k in std::iter::once(j).chain(ws.interface_bases(j)) {
                if !out.contains(&k) {
                    out.push(k);
                }
            }
        }
    }
    out
}

/// Writes the sets lowering worked out where the markers are (the incremental compile does not leave them to
/// link: the declarations did not change).
struct FillSets<'a>(&'a [super::localize::Expanded]);

impl al_ir::walk::RefVisitor for FillSets<'_> {
    fn object(&mut self, _: &mut usize) {}
    fn routine(&mut self, _: &mut usize, _: usize) {}
    fn field(&mut self, _: &mut usize, _: al_ir::walk::TableOf) {}
    fn control(&mut self, _: &mut usize) {}
    fn expansion(&mut self, index: usize, site: al_ir::walk::ExpansionSite<'_>) {
        self.0[index].fill(site);
    }
}

/// Takes what the page extensions add out of the pages that hold it: their controls, with where each goes, their
/// `modify` blocks and their page triggers. The page keeps its own controls. A page whose own controls refer to a
/// control of an extension is left as it is.
fn split_pages(staged: &mut [Staged], provided: &(dyn Fn(usize) -> bool + Sync), keys: &[al_ir::sym::ObjKey]) {
    use al_ir::sym::{ControlId, ControlMod, ExtControl, PageContribution};
    let n = staged.len();
    let mut contributions: Vec<Option<PageContribution>> = vec![None; n];
    #[allow(clippy::needless_range_loop)] // `b` is the object, which `provided` and `keys` take
    for b in 0..n {
        if provided(b) || staged[b].object.uncompiled || staged[b].object.page.is_none() {
            continue;
        }
        let mods = std::mem::take(&mut staged[b].page_mods);
        let mut page = staged[b].object.page.take().expect("checked");
        if page.controls.iter().all(|c| c.owner.is_none()) && page.ext_triggers.is_empty() && mods.mods.is_empty() {
            staged[b].object.page = Some(page);
            continue;
        }
        // Where each control goes: the page's own controls keep their order, an extension's are counted per extension.
        let mut pure_of: Vec<Option<usize>> = Vec::with_capacity(page.controls.len());
        let mut ext_at: Vec<Option<(usize, usize)>> = Vec::with_capacity(page.controls.len());
        let mut per_ext: Vec<(usize, usize)> = Vec::new();
        let mut pure = 0;
        for c in &page.controls {
            match c.owner {
                None => {
                    pure_of.push(Some(pure));
                    ext_at.push(None);
                    pure += 1;
                }
                Some(e) => {
                    let k = match per_ext.iter_mut().find(|(x, _)| *x == e) {
                        Some((_, k)) => {
                            *k += 1;
                            *k - 1
                        }
                        None => {
                            per_ext.push((e, 1));
                            0
                        }
                    };
                    pure_of.push(None);
                    ext_at.push(Some((e, k)));
                }
            }
        }
        let id_of = |i: usize| -> ControlId {
            match (pure_of[i], ext_at[i]) {
                (Some(p), _) => ControlId::Base(p),
                (None, Some((e, k))) => ControlId::Ext {
                    ext: keys[e].clone(),
                    index: k,
                },
                (None, None) => unreachable!("a control is the page's or an extension's"),
            }
        };
        // The page's own controls must not refer to an extension's.
        let own_refs_ext = page.controls.iter().enumerate().any(|(i, c)| {
            pure_of[i].is_some()
                && (c.parent.is_some_and(|p| pure_of[p].is_none())
                    || matches!(c.action_ref, Some(ir::ActionRef::Target(t)) if pure_of[t].is_none())
                    || c.part_link
                        .as_ref()
                        .is_some_and(|l| l.provider.is_some_and(|p| pure_of[p].is_none()))
                    || c.run_object
                        .as_ref()
                        .and_then(|r| r.link.as_ref())
                        .is_some_and(|l| l.provider.is_some_and(|p| pure_of[p].is_none())))
        });
        if own_refs_ext {
            staged[b].object.page = Some(page);
            continue;
        }
        // The properties of the controls the `modify` blocks change, before any applies.
        for (ci, before) in mods.before {
            page.controls[ci].props = before;
        }
        for c in &mut page.controls {
            c.ext_triggers.clear();
        }
        let area_of = |i: usize| {
            page.action_areas
                .iter()
                .find(|a| a.roots.contains(&i))
                .map(|a| a.name.clone())
        };
        let part_pages: Vec<Option<usize>> = page.controls.iter().map(|c| c.part_page).collect();
        let mut kept = Vec::with_capacity(pure);
        let old_controls = std::mem::take(&mut page.controls);
        let mut ext_controls: Vec<(usize, ExtControl)> = Vec::new();
        for (i, mut c) in old_controls.into_iter().enumerate() {
            let remap = |x: usize| pure_of[x].expect("checked");
            match ext_at[i] {
                None => {
                    c.parent = c.parent.map(remap);
                    if let Some(ir::ActionRef::Target(t)) = &mut c.action_ref {
                        *t = remap(*t);
                    }
                    if let Some(l) = c.part_link.as_mut() {
                        l.provider = l.provider.map(remap);
                    }
                    if let Some(l) = c.run_object.as_mut().and_then(|r| r.link.as_mut()) {
                        l.provider = l.provider.map(remap);
                    }
                    kept.push(c);
                }
                Some((e, _)) => {
                    let ext = ExtControl {
                        pred: (i > 0).then(|| id_of(i - 1)),
                        parent: c.parent.map(&id_of),
                        target: match c.action_ref {
                            Some(ir::ActionRef::Target(t)) => Some(id_of(t)),
                            _ => None,
                        },
                        provider: c.part_link.as_ref().and_then(|l| l.provider).map(&id_of),
                        area: if c.parent.is_none() { area_of(i) } else { None },
                        provider_page: c
                            .part_link
                            .as_ref()
                            .and_then(|l| l.provider)
                            .and_then(|p| part_pages[p]),
                        control: {
                            c.parent = None;
                            if let Some(ir::ActionRef::Target(t)) = &mut c.action_ref {
                                *t = 0;
                            }
                            if let Some(l) = c.part_link.as_mut() {
                                l.provider = None;
                            }
                            c
                        },
                    };
                    ext_controls.push((e, ext));
                }
            }
        }
        page.controls = kept;
        for a in &mut page.action_areas {
            a.roots = a.roots.iter().filter_map(|&r| pure_of[r]).collect();
        }
        for (e, c) in ext_controls {
            contributions[e].get_or_insert_with(Default::default).controls.push(c);
        }
        for m in mods.mods {
            contributions[m.ext]
                .get_or_insert_with(Default::default)
                .mods
                .push(ControlMod {
                    target: id_of(m.control),
                    props: m.props,
                    triggers: m.triggers,
                });
        }
        for (name, e, r) in std::mem::take(&mut page.ext_triggers) {
            contributions[e]
                .get_or_insert_with(Default::default)
                .triggers
                .push((name, r));
        }
        staged[b].object.page = Some(page);
    }
    for (e, c) in contributions.into_iter().enumerate() {
        staged[e].page_contribution = c;
    }
}

/// A cache block's refusals and direct calls, per routine.
type BlockMeta = (Vec<Vec<String>>, Vec<Vec<(u32, u32)>>);

/// What making the objects symbolic and linking them gave: the program, the program as lowering used to build it
/// (the check), and the symbolic objects (for a cache).
struct Linking {
    linked: al_link::Linked,
    expected: Option<Expected>,
    syms: Vec<Option<al_ir::sym::SymObject>>,
}

/// What `link_pieces` is asked to do besides linking.
struct LinkOptions {
    check: bool,
    export: bool,
}

/// The program built the way lowering used to build it, and where each object's source positions are (the check).
type Expected = (ir::Program, Vec<(u32, u32)>);

/// A lowered object waiting to be linked.
struct Staged {
    object: ir::Object,
    positions: Vec<ir::SrcPos>,
    key: Option<al_ir::sym::ObjKey>,
    field_refs: Vec<(usize, u32)>,
    control_refs: Vec<(usize, String, usize)>,
    expansions: Vec<super::localize::Expanded>,
    page_mods: super::localize::PageMods,
    /// A provided object in its symbolic form.
    sym: Option<al_ir::sym::SymObject>,
    /// What a page extension adds to its page, taken out of the page before the objects are made symbolic.
    page_contribution: Option<al_ir::sym::PageContribution>,
    /// A provided codeunit: the interfaces it implements, from the cache's facts.
    implements: Vec<al_ir::sym::ObjKey>,
    enum_impls: Vec<al_ir::sym::EnumImplDecl>,
    enum_ordinals: Vec<u32>,
    key_owners: Vec<usize>,
    /// What a table extension adds to its table, taken out of the table before the objects are made symbolic.
    table_contribution: Option<al_ir::sym::TableContribution>,
    /// For a provided table: its field numbers (`DepFacts::fields`).
    fields: Vec<u32>,
    /// For a provided page: its control names (`DepFacts::controls`).
    controls: Vec<String>,
}

/// Turns the lowered objects, in program order, into the program (ADR-033): the objects that were
/// lowered now become symbolic, the ones a compile cache supplied (`provided`) are placed as they
/// are, and link gives every object its position. `base` holds everything that is not an object.
/// With `check`, the program is also built the way lowering used to build it, and a difference
/// is an error (`AL_CHECK_LINK=1`).
fn link_pieces(
    mut staged: Vec<Staged>,
    base: ir::Program,
    provided: &(dyn Fn(usize) -> bool + Sync),
    routine_keys: &super::localize::RoutineKeys<'_>,
    lookups: &Lookups<'_>,
    subscribers: Vec<Vec<al_ir::sym::SubscriberDecl>>,
    options: LinkOptions,
) -> Result<Linking, String> {
    let LinkOptions { check, export } = options;
    use al_ir::sym::ObjKey;
    let keys: Vec<ObjKey> = staged
        .iter()
        .map(|s| s.key.clone().unwrap_or_else(|| ObjKey::of(&s.object, &base.apps)))
        .collect();
    // The check builds the program the way lowering used to: every reference a position.
    // (Objects a cache supplies are symbolic already: the cached and the complete compile are compared
    // as programs by the tests of `provided.rs` instead.)
    let any_sym = staged.iter().any(|s| s.sym.is_some());
    let expected = (check && !any_sym).then(|| {
        let mut p = base.clone();
        let mut ranges = Vec::new();
        for (oi, s) in staged.iter().enumerate() {
            let at = p.positions.len() as u32;
            ranges.push((at, s.positions.len() as u32));
            let mut object = s.object.clone();
            if !provided(oi) {
                let mut u = Unembed {
                    expansions: &s.expansions,
                    list: &s.field_refs,
                    controls: &s.control_refs,
                    kinds: &keys.iter().map(|k| k.kind).collect::<Vec<_>>(),
                    field_pos: lookups.field_pos,
                };
                al_ir::walk::walk_object(&mut object, &mut u);
            }
            if !s.positions.is_empty() {
                for r in &mut object.routines {
                    r.shift_positions(at);
                }
                p.positions.extend(s.positions.iter().copied());
            }
            p.objects.push(object);
        }
        (p, ranges)
    });
    let routines_of = |i: usize| routine_keys.of(i);
    let tables = super::localize::Keys {
        objects: &keys,
        routines: &routines_of,
        table_of: lookups.table_of,
        field_no: lookups.field_no,
        report_item_table: lookups.report_item_table,
        extends: lookups.extends,
        join_order: lookups.join_order,
        own_enum: lookups.own_enum,
        implements: lookups.implements,
        enum_info: lookups.enum_info,
    };
    let workers = lower_workers();
    // The lists of extensions the objects hold, as the lists the extension objects are in.
    let mut joins: Vec<Vec<(al_ir::sym::ListKind, usize)>> = vec![Vec::new(); staged.len()];
    for (b, s) in staged.iter().enumerate().filter(|(b, _)| !provided(*b)) {
        let lists = [
            (
                al_ir::sym::ListKind::TableExtensions,
                s.object.table.as_ref().map(|t| &t.extensions),
            ),
            (
                al_ir::sym::ListKind::PageExtensions,
                s.object.page.as_ref().map(|p| &p.extensions),
            ),
            (
                al_ir::sym::ListKind::ReportExtensions,
                s.object.report.as_ref().map(|r| &r.extensions),
            ),
        ];
        for (kind, list) in lists {
            for &e in list.into_iter().flatten() {
                joins[e].push((kind, b));
            }
        }
    }
    split_tables(&mut staged, provided, lookups, export);
    split_pages(&mut staged, provided, &keys);
    let joins = &joins;
    let mut units: Vec<al_ir::sym::SymObject> = al_link::par_map(staged, workers, |oi, s| {
        if let Some(sym) = s.sym {
            Ok(sym)
        } else if provided(oi) {
            // Only the facts the other objects ask of it.
            Ok(al_ir::sym::SymObject {
                key: keys[oi].clone(),
                placeholder: true,
                enum_impls: s.enum_impls,
                enum_ordinals: s.enum_ordinals,
                implements: s.implements,
                field_numbers: s.fields,
                control_names: s.controls,
                ..Default::default()
            })
        } else {
            super::localize::localize(
                &tables,
                oi,
                s.object,
                s.positions,
                &s.field_refs,
                &s.control_refs,
                super::localize::Extras {
                    joins: &joins[oi],
                    table_contribution: s.table_contribution,
                    page_contribution: s.page_contribution,
                    expansions: &s.expansions,
                    subscribers: subscribers[oi].clone(),
                },
            )
        }
    })
    .into_iter()
    .collect::<Result<_, String>>()?;
    // A table nothing compiled has no field list, but other objects may name its fields: they are
    // counted from its members, for the tables that are named.
    // Only such a table can take the numbers. A cached run has none (every dependency table comes
    // compiled), so the walk over every reference of every object is skipped then.
    let unlisted = units
        .iter()
        .zip(&keys)
        .any(|(s, k)| k.kind == ObjKind::Table && s.object.table.is_none() && !s.placeholder);
    let mut numbered = std::collections::HashSet::new();
    if unlisted {
        let at: HashMap<&ObjKey, usize> = keys.iter().enumerate().map(|(i, k)| (k, i)).collect();
        for s in &units {
            for f in &s.refs.fields {
                let table = &s.refs.objects[f.table as usize];
                if table.kind == ObjKind::Table {
                    if let Some(&t) = at.get(table) {
                        numbered.insert(t);
                    }
                }
            }
        }
    }
    for t in numbered {
        let s = &mut units[t];
        if s.object.table.is_none() && !s.placeholder {
            s.field_numbers = (lookups.field_numbers)(t);
        }
    }
    let syms: Vec<Option<al_ir::sym::SymObject>> = if export {
        units.iter().map(|u| (!u.placeholder).then(|| u.clone())).collect()
    } else {
        Vec::new()
    };
    let mut base = base;
    base.uncompiled_subscribers.clear();
    let linked = al_link::link(units, base, workers, &routines_of).map_err(|e| format!("link: {e}"))?;
    Ok(Linking { linked, expected, syms })
}

/// The incremental compile (`incremental.rs`): the objects `dirty` changed in code only (the declarations of
/// their files are as they were), so only they are checked and lowered again, and `old` (the lowering of the
/// workspace before the change, with the dependency objects of `src`) takes the result in place. The program
/// afterwards is the one a complete compile of the changed workspace gives.
///
/// Why the other objects stay as they are: an object's lowering reads its own syntax and the declarations of
/// other objects, never their code, and every cross-object reference in the IR is an object or routine
/// number, which a change of code leaves as it is. The caller allows only the kinds of object for which that
/// holds (`incremental::eligible`).
pub fn relower_objects(
    ws: &Workspace,
    catalog: &al_builtins::catalog::Catalog,
    roots: crate::check::Roots,
    src: &dyn crate::provided::DepSource,
    old: &mut Lowered,
    dirty: &[usize],
) -> Result<(), String> {
    let n = ws.objects.len();
    if old.program.objects.len() != n || old.pos_ranges.len() != n {
        return Err("the earlier compile does not fit the workspace".into());
    }
    let t0 = std::time::Instant::now();
    let members = crate::members::build(ws);
    let started = std::time::Instant::now();
    let mut roots = roots;
    roots.eager = true;
    roots.provided = old.provided.clone();
    roots.provided_queries = (0..n)
        .map(|oi| old.provided[oi].then(|| src.facts(oi).query.clone()).flatten())
        .collect();
    roots.only_objects = Some(dirty.to_vec());
    roots.skipped_before = Some(old.skipped.clone());
    let checked = crate::check::check_with(ws, &members, catalog, roots);
    let check_time = started.elapsed();
    let started = std::time::Instant::now();
    let flags = old.provided.clone();
    let Prologue {
        mut shared,
        uncompiled_subscribers,
    } = build_shared(ws, &members, &checked, catalog, true, Some(src), &flags, false);
    // The objects go into the program as they are lowered: nothing works out the relations afterwards.
    shared.early_relations = true;
    let shared_time = started.elapsed();
    let pieces: Vec<(usize, Piece)> = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(LOWER_STACK)
            .spawn_scoped(scope, || {
                let local = Local::new(Nodes::new(ws, shared.active.clone()));
                let seq = Seq::new(&shared);
                let mut line_cache: HashMap<usize, std::rc::Rc<FileLines>> = HashMap::default();
                dirty
                    .iter()
                    .map(|&oi| (oi, lower_object(&shared, &local, &seq, oi, &mut line_cache)))
                    .collect()
            })
            .expect("spawn a lowering thread")
            .join()
            .expect("the lowering thread panicked")
    });
    // The new objects are made symbolic and linked into the program as it is (ADR-033); link also
    // numbers the source positions again, in object order, the objects that did not change moved
    // to their new place.
    let program = std::sync::Arc::make_mut(&mut old.program);
    let keys: Vec<al_ir::sym::ObjKey> = program
        .objects
        .iter()
        .map(|o| al_ir::sym::ObjKey::of(o, &program.apps))
        .collect();
    let compiled: Vec<bool> = program.objects.iter().map(|o| !o.uncompiled).collect();
    let routine_keys = super::localize::RoutineKeys::new(
        ws,
        (0..n)
            .map(|i| old.provided[i].then(|| src.facts(i).routine_keys.as_slice()))
            .collect(),
        compiled,
    );
    let routines_of = |i: usize| routine_keys.of(i);
    let field_no = |t: usize, pos: usize| members.get(t).fields.get(pos).map(|f| f.id);
    let report_item_table = |r: usize, item: usize| {
        shared
            .report_irs
            .get(r)
            .and_then(|x| x.as_ref())
            .and_then(|x| x.items.get(item))
            .and_then(|i| i.table)
    };
    let tables = super::localize::Keys {
        objects: &keys,
        routines: &routines_of,
        table_of: &shared.table_of,
        field_no: &field_no,
        report_item_table: &report_item_table,
        extends: &|e| ws.extension_target(e),
        implements: &|cu| implemented_interfaces(ws, cu),
        enum_info: &|i| enum_info_of(ws, &members, i),
        join_order: &|_, _| (0, 0, 0),
        own_enum: &|i| {
            let m = members.own(i);
            (
                m.enum_values.iter().map(|(v, n)| (*v as i32, n.clone())).collect(),
                m.enum_captions.iter().map(|(v, c)| (*v as i32, c.clone())).collect(),
            )
        },
    };
    let mut replace = Vec::with_capacity(pieces.len());
    for (oi, mut piece) in pieces {
        al_ir::walk::walk_object(&mut piece.object, &mut FillSets(&piece.expansions));
        old.unsupported[oi] = piece.unsupported;
        old.calls[oi] = piece.calls;
        replace.push((
            oi,
            super::localize::localize(
                &tables,
                oi,
                piece.object,
                piece.positions,
                &piece.field_refs,
                &piece.control_refs,
                // The lists of extensions stay as they were: the declarations did not change.
                super::localize::Extras {
                    joins: &[],
                    table_contribution: None,
                    page_contribution: None,
                    expansions: &[],
                    subscribers: Vec::new(),
                },
            )?,
        ));
    }
    al_link::relink(program, &mut old.pos_ranges, &keys, &routines_of, replace).map_err(|e| format!("link: {e}"))?;
    program.uncompiled_subscribers = uncompiled_subscribers;
    // `--tdd` (ADR-051 decision 9): the stubs of the objects checked again replace theirs; the others stay.
    if checked.tdd {
        old.tdd_sites.retain(|(oi, _)| !dirty.contains(oi));
        old.tdd_sites.extend(
            crate::check::tdd::sites_by_object(ws, &checked.resolutions)
                .into_iter()
                .filter(|(oi, _)| dirty.contains(oi)),
        );
        program.tdd_stubs = tdd_stub_list(&checked, &old.tdd_sites);
        old.tdd_needs = crate::check::tdd_decl::needs(ws, &checked);
    }
    old.driven = Some(shared.driven.clone());
    // The diagnostics: what the checker found in the files that did not change stays.
    let dirty_files: std::collections::HashSet<&str> = dirty
        .iter()
        .map(|&oi| ws.files[ws.objects[oi].file].path.as_str())
        .collect();
    let mut from_checker: Vec<crate::workspace::Diagnostic> = std::mem::take(&mut old.checker_diags);
    from_checker.retain(|d| !dirty_files.contains(d.file.as_str()));
    from_checker.extend(checked.from_checker.iter().cloned());
    let mut all: Vec<crate::workspace::Diagnostic> = from_checker.clone();
    all.extend(checked.from_codes.iter().cloned());
    let order: HashMap<&str, usize> = ws.files.iter().enumerate().map(|(i, f)| (f.path.as_str(), i)).collect();
    all.sort_by_key(|d| (order.get(d.file.as_str()).copied(), d.offset));
    old.al_diagnostics = all.into_iter().filter(|d| d.code.starts_with("AL")).collect();
    old.checker_diags = from_checker;
    old.skipped = checked.skipped.clone();
    old.timings = CompileTimings {
        check: checked.timings,
        lower: started.elapsed(),
        ..Default::default()
    };
    if std::env::var_os("ALRUNNER_INCR_TIMINGS").is_some() {
        eprintln!(
            "incremental: check {:.3} s (members {:.3}), shared tables {:.3} s, then lower and patch {:.3} s; total {:.3} s",
            check_time.as_secs_f64(),
            0.0,
            shared_time.as_secs_f64(),
            started.elapsed().as_secs_f64() - shared_time.as_secs_f64(),
            t0.elapsed().as_secs_f64()
        );
    }
    Ok(())
}

/// The lowering of one object: its IR, why each routine cannot run, and the routines each calls.
pub(crate) struct Piece {
    pub(crate) object: ir::Object,
    /// The source positions of its statements, numbered from 0.
    pub(crate) positions: Vec<ir::SrcPos>,
    pub(crate) unsupported: Vec<Vec<String>>,
    pub(crate) calls: Vec<Vec<(usize, usize)>>,
    /// With `Shared::export`: `calls` without what a page, report or xmlport variable drives.
    pub(crate) direct: Vec<Vec<(usize, usize)>>,
    /// The objects whose extensions or implementers the lowering read (see `provided.rs`).
    pub(crate) embeds: Vec<usize>,
    /// With a compile cache: the key the cache recorded for the object (a layer build's placeholder has no object to take it from).
    pub(crate) key: Option<al_ir::sym::ObjKey>,
    /// The fields the object's expressions name: (table, field number) (`localize.rs`).
    pub(crate) field_refs: Vec<(usize, u32)>,
    /// The controls the object's expressions name (`localize.rs`).
    pub(crate) control_refs: Vec<(usize, String, usize)>,
    /// The sets that link works out for the object's sites (`localize::Expanded`).
    pub(crate) expansions: Vec<super::localize::Expanded>,
    /// What the page extensions' `modify` blocks change in the page's controls.
    pub(crate) page_mods: super::localize::PageMods,
    /// A provided object in its symbolic form (what the cache holds).
    pub(crate) sym: Option<al_ir::sym::SymObject>,
    /// The object that declares each key of a table (the table or one of its extensions).
    pub(crate) key_owners: Vec<usize>,
    /// A provided codeunit: the interfaces it implements, from the cache's facts.
    pub(crate) implements: Vec<al_ir::sym::ObjKey>,
    pub(crate) enum_impls: Vec<al_ir::sym::EnumImplDecl>,
    pub(crate) enum_ordinals: Vec<u32>,
    /// A provided table: its field numbers, from the cache's facts.
    pub(crate) fields: Vec<u32>,
    /// A provided page: its control names, from the cache's facts.
    pub(crate) controls: Vec<String>,
}

/// How many threads lower dependency objects. `AL_LOWER_THREADS` sets it; the default is the
/// number of cores.
pub(crate) fn lower_workers() -> usize {
    std::env::var("AL_LOWER_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
        .clamp(1, 32)
}

/// A thread is started for at least this many objects.
pub(crate) const PARALLEL_OBJECTS_PER_WORKER: usize = 16;
/// Objects a lowering worker takes at a time.
pub(crate) const LOWER_CHUNK: usize = 4;
/// Stack of a lowering worker: lowering recurses on the nesting of the code.
pub(crate) const LOWER_STACK: usize = 64 << 20;

/// `--tdd`: every stub of the program, sorted, each once: the stub procedures and the stub declarations
/// (`ir::Program::tdd_stubs`).
fn tdd_stub_list(checked: &CheckResult, sites: &[(usize, String)]) -> Vec<String> {
    let mut all = crate::check::tdd::stub_list(sites);
    if let Some(d) = &checked.tdd_decls {
        all.extend(d.all.iter().cloned());
        all.sort();
        all.dedup();
    }
    all
}
