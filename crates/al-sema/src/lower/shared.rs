//! What the lowering of an object reads (plain data built once) and the per-object, per-thread state,
//! with the constants and literal helpers the lowerer uses.
use super::*;

/// What the lowering of an object reads that is plain data, built once and read by every thread.
pub(crate) struct Shared<'a> {
    pub(crate) ws: &'a Workspace,
    pub(crate) members: &'a MemberTable<'a>,
    pub(crate) checked: &'a CheckResult,
    pub(crate) catalog: &'a al_builtins::catalog::Catalog,
    pub(crate) demand: Option<&'a crate::check::demand::Demand>,
    /// Statements carry source positions (`lower_with_positions`).
    pub(crate) positions: bool,
    /// See `Lowerer::early_relations`.
    pub(crate) early_relations: bool,
    /// Per object: it is part of the compile (see `Workspace::restrict_to`).
    pub(crate) active: Vec<bool>,
    /// The table whose record is the implicit `Rec` of each object.
    pub(crate) table_of: Vec<Option<usize>>,
    pub(crate) page_temporary: Vec<bool>,
    pub(crate) page_reasons: Vec<Vec<String>>,
    /// What a variable of a page, report or xmlport type drives: the page extensions of each
    /// page, the parts and actions of each page, the report extension of each report.
    pub(crate) driven: DrivenCtx,
    pub(crate) xml_irs: Vec<Option<ir::XmlPort>>,
    pub(crate) report_irs: Vec<Option<ir::Report>>,
    /// Per object: the first reason a variable of the xmlport or report is refused.
    pub(crate) xml_reasons: Vec<Option<String>>,
    pub(crate) report_reasons: Vec<Option<String>>,
    pub(crate) ext_irs: Vec<Option<ir::ReportExt>>,
    /// The global slot of each variable of a page extension, by lower-case name.
    pub(crate) ext_slots: Vec<HashMap<String, usize>>,
    /// The start offset of every routine, numbered as `Nodes` numbers them.
    pub(crate) routine_offsets: std::sync::Arc<Vec<Vec<usize>>>,
    pub(crate) routine_at: RoutineAt,
    /// The extensions of each table.
    pub(crate) extensions: HashMap<usize, Vec<usize>>,
    /// For a report extension that changes a run, the report it belongs to.
    pub(crate) ext_base: Vec<Option<usize>>,
    /// Keep what a cache needs (`Lowered::exported`).
    pub(crate) export: bool,
    /// The parse diagnostics (`AR0000`) of `ws.diagnostics` by file path, in their order: each object asks for those of
    /// its file, and a scan of every diagnostic per object grows with the number of objects times diagnostics.
    pub(crate) parse_diags: HashMap<&'a str, Vec<&'a crate::workspace::Diagnostic>>,
}

/// The routine nodes of the objects, built when first asked for. A syntax tree belongs to the
/// thread that made it, so each thread has its own.
pub(crate) struct Nodes<'a> {
    pub(crate) ws: &'a Workspace,
    pub(crate) active: Vec<bool>,
    pub(crate) cells: Vec<std::cell::OnceCell<Vec<ast::Routine>>>,
}

impl<'a> Nodes<'a> {
    pub(crate) fn new(ws: &'a Workspace, active: Vec<bool>) -> Nodes<'a> {
        Nodes {
            ws,
            active,
            cells: (0..ws.objects.len()).map(|_| std::cell::OnceCell::new()).collect(),
        }
    }

    /// The procedures and triggers of object `oi` in source order (none if it is not compiled).
    pub(crate) fn get(&self, oi: usize) -> &[ast::Routine] {
        self.cells[oi].get_or_init(|| {
            if self.active[oi] {
                ast::routines_in(self.ws.object_ast(oi).syntax())
            } else {
                Vec::new()
            }
        })
    }
}

/// What one thread keeps while it lowers objects.
pub(crate) struct Local<'a> {
    pub(crate) nodes: Nodes<'a>,
    pub(crate) memo: Memo,
}

impl<'a> Local<'a> {
    pub(crate) fn new(nodes: Nodes<'a>) -> Local<'a> {
        Local {
            nodes,
            memo: Memo::default(),
        }
    }
}

/// The layouts of pages, report request pages, XML ports and report extensions, built from the
/// syntax tree of an object when first asked for. The prologue of `lower_with_positions` builds
/// the same layouts to find what the objects need of each other; a layout holds syntax nodes,
/// which belong to the thread that made them, so each thread builds the ones it reads.
pub(crate) struct Seq<'a> {
    pub(crate) ws: &'a Workspace,
    pub(crate) active: &'a [bool],
    pub(crate) page_extensions: &'a HashMap<usize, Vec<usize>>,
    pub(crate) ext_base: &'a [Option<usize>],
    pub(crate) page: Vec<std::cell::OnceCell<Option<crate::pages::Layout>>>,
    pub(crate) xml: Vec<std::cell::OnceCell<Option<crate::xmlports::Layout>>>,
    pub(crate) report: Vec<std::cell::OnceCell<Option<crate::reports::Layout>>>,
    pub(crate) ext: Vec<std::cell::OnceCell<Option<crate::reports::ExtLayout>>>,
}

impl<'a> Seq<'a> {
    pub(crate) fn new(sh: &'a Shared<'a>) -> Seq<'a> {
        fn cells<T>(n: usize) -> Vec<std::cell::OnceCell<T>> {
            (0..n).map(|_| std::cell::OnceCell::new()).collect()
        }
        let n = sh.ws.objects.len();
        Seq {
            ws: sh.ws,
            active: &sh.active,
            page_extensions: &sh.driven.page_extensions,
            ext_base: &sh.ext_base,
            page: cells(n),
            xml: cells(n),
            report: cells(n),
            ext: cells(n),
        }
    }

    /// The extensions of page or report `oi` with their syntax trees.
    pub(crate) fn extensions(&self, oi: usize) -> Vec<(usize, ast::Object)> {
        self.page_extensions
            .get(&oi)
            .map(|v| v.iter().map(|&e| (e, self.ws.object_ast(e))).collect())
            .unwrap_or_default()
    }

    /// The control tree of a page; of a report, its request page.
    pub(crate) fn page_layout(&self, oi: usize) -> Option<&crate::pages::Layout> {
        self.page[oi]
            .get_or_init(|| match self.ws.objects[oi].kind {
                ObjKind::Page if self.active[oi] => {
                    Some(crate::pages::layout_with(&self.ws.object_ast(oi), &self.extensions(oi)))
                }
                ObjKind::Report if self.active[oi] => {
                    let l = self.report(oi)?;
                    Some(crate::reports::request_layout(&self.ws.object_ast(oi), &l.items, &self.extensions(oi)).0)
                }
                _ => None,
            })
            .as_ref()
    }

    pub(crate) fn xml(&self, oi: usize) -> Option<&crate::xmlports::Layout> {
        self.xml[oi]
            .get_or_init(|| {
                (self.ws.objects[oi].kind == ObjKind::XmlPort && self.active[oi])
                    .then(|| crate::xmlports::layout(&self.ws.object_ast(oi)))
            })
            .as_ref()
    }

    pub(crate) fn report(&self, oi: usize) -> Option<&crate::reports::Layout> {
        self.report[oi]
            .get_or_init(|| {
                (self.ws.objects[oi].kind == ObjKind::Report && self.active[oi])
                    .then(|| crate::reports::layout(&self.ws.object_ast(oi)))
            })
            .as_ref()
    }

    /// The report a report extension that changes a run belongs to.
    pub(crate) fn ext_base(&self, oi: usize) -> Option<usize> {
        self.ext_base[oi]
    }

    pub(crate) fn ext(&self, oi: usize) -> Option<&crate::reports::ExtLayout> {
        self.ext[oi]
            .get_or_init(|| self.ext_base[oi].map(|_| crate::reports::ext_layout(&self.ws.object_ast(oi))))
            .as_ref()
    }
}

pub(crate) fn make_lowerer<'a>(sh: &'a Shared<'a>, tl: &'a Local<'a>, seq: &'a Seq<'a>, oi: usize) -> Lowerer<'a> {
    Lowerer {
        rec_slot: None,
        table_of: &sh.table_of,
        ws: sh.ws,
        members: sh.members,
        checked: sh.checked,
        catalog: sh.catalog,
        routine_at: &sh.routine_at,
        oi,
        file: sh.ws.objects[oi].file,
        globals: Default::default(),
        request_page_globals: Default::default(),
        in_request_page: false,
        in_report_init_with_rec: None,
        locals: Default::default(),
        local_char_option_byte_vars: Default::default(),
        global_labels: HashMap::default(),
        local_labels: HashMap::default(),
        global_label_xliff: HashMap::default(),
        local_label_xliff: HashMap::default(),
        global_textconsts: Default::default(),
        local_textconsts: Default::default(),
        global_textconst_languages: HashMap::default(),
        local_textconst_languages: HashMap::default(),
        global_locked_labels: Default::default(),
        local_locked_labels: Default::default(),
        global_maxlength_labels: Default::default(),
        local_maxlength_labels: Default::default(),
        global_var_attrs: HashMap::default(),
        local_var_attrs: HashMap::default(),
        run_refusals: Default::default(),
        run_failures: Default::default(),
        with_places: HashMap::default(),
        reasons: Vec::new(),
        refusal_at: None,
        calls: Vec::new(),
        page_reasons: &sh.page_reasons,
        layouts: seq,
        xml_reasons: &sh.xml_reasons,
        report_reasons: &sh.report_reasons,
        memo: &tl.memo,
        ret_ty: None,
        qcp_ret: None,
        hidden_locals: Vec::new(),
        hidden_base: 0,
        curr_field_args: None,
        stream_scope: Vec::new(),
        loops: Vec::new(),
        var_params: Vec::new(),
        reads_var_max_len: false,
        ret_decl: None,
        routine_nodes: &tl.nodes,
        ext_slots: &sh.ext_slots,
        page_extensions: &sh.driven.page_extensions,
        embeds: Default::default(),
        field_refs: Default::default(),
        control_refs: Default::default(),
        expansions: Default::default(),
        page_mods: Default::default(),
        early_relations: sh.early_relations,
        positions: Vec::new(),
        lines: None,
        emit_pos: false,
        routine_idx: 0,
        assumed: Vec::new(),
        tdd_marked: Default::default(),
        tdd_globals: HashMap::default(),
        tys: Vec::new(),
        tys_on: false,
        roots: Vec::new(),
    }
}

/// Records what a global's type says about its text: label text and XLIFF id, textconst, languages, `Locked`, `MaxLength`.
fn note_global_label_facts(lw: &mut Lowerer<'_>, v: &ast::VarDecl, n: &str, label: Option<&String>) {
    let key = n.to_lowercase();
    if let Some(l) = label {
        lw.global_labels.insert(key.clone(), l.clone());
        if let Some(id) = lw.label_xliff_id(None, n) {
            lw.global_label_xliff.insert(key.clone(), id);
        }
    }
    if is_textconst(v.type_ref().as_ref()) {
        lw.global_textconsts.insert(key.clone());
    }
    if let Some(texts) = v.type_ref().as_ref().and_then(textconst_languages) {
        lw.global_textconst_languages.insert(key.clone(), texts);
    }
    if v.type_ref().as_ref().is_some_and(label_locked) {
        lw.global_locked_labels.insert(key.clone());
    }
    if v.type_ref().as_ref().is_some_and(label_maxlength) {
        lw.global_maxlength_labels.insert(key);
    }
}

/// Lowers object `oi`. `positions` and `line_cache` are the source positions and file line tables
/// of the objects lowered so far on this thread.
pub(crate) fn lower_object<'a>(
    sh: &'a Shared<'a>,
    tl: &'a Local<'a>,
    seq: &'a Seq<'a>,
    oi: usize,
    line_cache: &mut HashMap<usize, std::rc::Rc<FileLines>>,
) -> Piece {
    crate::halt::check(); // A hard memory limit stops the compile at the next object (`halt.rs`).
    let ws = sh.ws;
    let members = sh.members;
    let checked = sh.checked;
    let demand = sh.demand;
    let table_of = &sh.table_of;
    let page_temporary = &sh.page_temporary;
    let page_extensions = &sh.driven.page_extensions;
    let page_reasons = &sh.page_reasons;
    let xml_irs = &sh.xml_irs;
    let report_irs = &sh.report_irs;
    let ext_irs = &sh.ext_irs;
    let routine_at = &sh.routine_at;
    let routine_offsets = &sh.routine_offsets;
    let extensions = &sh.extensions;
    let o = &ws.objects[oi];
    if !sh.active[oi] {
        // Not compiled: a shell keeps object numbers stable. Its kind is `uncompiled table` and
        // so on, so nothing the runtime looks up by kind and id (`Codeunit.Run(80)`) finds it
        // and runs it as an empty object; the few lookups that would answer "no such object"
        // (`RecordRef.Open(18)`) look for the shell and refuse instead.
        return Piece {
            lowered_routines: 0,
            object: ir::Object {
                kind: o.kind,
                uncompiled: true,
                id: o.id,
                name: o.name.clone(),
                globals: Vec::new(),
                routines: Vec::new(),
                // What the declaration scan read of the object body, so the metadata tables can
                // answer for an object nothing compiled.
                properties: with_profile_target(ws, oi, o.scanned_properties.clone()),
                enum_values: Vec::new(),
                enum_captions: Vec::new(),
                enum_captions_ml: Vec::new(),
                table: None,
                rec_slot: None,
                xrec_slot: None,
                page: None,
                app: o.app as u32,
                query: None,
                xmlport: None,
                report: None,
                report_ext: None,
                namespace: o.namespace.clone(),
                extends_id: ws.extension_target(oi).and_then(|t| ws.objects[t].id),
                layouts: o
                    .scanned_layouts
                    .iter()
                    .map(|l| ir::LayoutDecl {
                        name: l.name.clone(),
                        properties: l.properties.clone(),
                    })
                    .collect(),
            },
            positions: Vec::new(),
            unsupported: Vec::new(),
            refusal_at: Vec::new(),
            calls: Vec::new(),
            direct: Vec::new(),
            ndirect: Vec::new(),
            embeds: Vec::new(),
            key: None,
            field_refs: Vec::new(),
            control_refs: Vec::new(),
            expansions: Vec::new(),
            page_mods: Default::default(),
            sym: None,
            key_owners: Vec::new(),
        };
    }
    let obj = ws.object_ast(oi);
    let mut lw = make_lowerer(sh, tl, seq, oi);
    // Positions feed call stacks and error lines. A dependency app's routines carry them too (ADR-041): Microsoft's Error
    // Message Management reads `SessionInformation.Callstack` and every frame needs its line. `AL_NO_DEP_POSITIONS` turns
    // that off (ADR-013 measured positions at a third of the lowering of a dependency app).
    lw.lines = (sh.positions && (!ws.is_dependency(oi) || dependency_positions())).then(|| {
        line_cache
            .entry(o.file)
            .or_insert_with(|| std::rc::Rc::new(FileLines::new(ws, o.file)))
            .clone()
    });
    let mut globals = Vec::new();
    let mut global_reasons = Vec::new();
    // An `#undef` of a symbol defined only in another case (the preprocessor reports it, see `al_syntax::preprocessor`):
    // which branches the file keeps is not pinned, so nothing in it runs.
    let path = &ws.files[o.file].path;
    let parse_diags = sh.parse_diags.get(path.as_str()).map(Vec::as_slice).unwrap_or_default();
    if let Some(d) = parse_diags
        .iter()
        .find(|d| d.message.starts_with("#undef ") && d.message.contains("only in another case"))
    {
        global_reasons.push(d.message.clone());
    } else if !ws.is_dependency(oi) {
        // Source of an app under test that our parser did not accept. It is not a compile error: the parser can reject
        // valid AL (al-runner issue 5576), so nothing in the file runs and the tests that need it are not run, with the
        // parse error as the reason (orchestrator decision 2026-10-21). A dependency app's file is handled by its load.
        if let Some(d) = parse_diags.iter().find(|d| d.app == ws.files[o.file].app) {
            global_reasons.push(format!("the parser did not accept {}: {}", d.file, d.message));
        }
    }
    if let Some(l) = seq.xml(oi) {
        for n in &l.nodes {
            let (ty, table) = match n.kind {
                // xmlports/shapes-properties/XmlPortBigText.al:54: `TextType = BigText` makes the variable a BigText; on a
                // text attribute too (reports/runtime/UPropertyTests.al:311-330).
                ir::XmlNodeKind::Text | ir::XmlNodeKind::TextAttr
                    if n.section.properties().any(|p| {
                        p.name().eq_ignore_ascii_case("TextType")
                            && p.value_text().trim().eq_ignore_ascii_case("BigText")
                    }) =>
                {
                    (
                        Ok(RtType::Library(al_builtins::library_kind::LibraryKind::BigText)),
                        false,
                    )
                }
                ir::XmlNodeKind::Text | ir::XmlNodeKind::TextAttr => (Ok(RtType::Text(None)), false),
                ir::XmlNodeKind::Table => (
                    lw.rt(&Ty::Record {
                        table: n.args.get(1).cloned().unwrap_or_default().into(),
                        temporary: n.section.properties().any(|p| {
                            p.name().eq_ignore_ascii_case("UseTemporary")
                                && p.value_text().trim().eq_ignore_ascii_case("true")
                        }),
                    }),
                    true,
                ),
                _ => continue,
            };
            let key = n.name.to_lowercase();
            if lw.globals.contains_key(&key) {
                continue;
            }
            lw.globals.insert(key, globals.len());
            match ty {
                Ok(rt) => globals.push(ir::Slot::new(n.name.clone(), rt)),
                Err(e) => {
                    global_reasons.push(format!(
                        "{} `{}`: {e}",
                        if table { "table element" } else { "variable" },
                        n.name
                    ));
                    globals.push(ir::Slot::new(n.name.clone(), RtType::Variant));
                }
            }
        }
        for name in ["__source", "__destination"] {
            let ty = if name == "__source" { "InStream" } else { "OutStream" };
            globals.push(ir::Slot::new(name, RtType::Library(ty.into())));
        }
    }
    if let Some(l) = seq.report(oi) {
        for i in &l.items {
            let key = i.name.to_lowercase();
            if lw.globals.contains_key(&key) {
                continue;
            }
            let ty = lw.rt(&Ty::Record {
                table: i.table.as_str().into(),
                temporary: crate::reports::uses_temporary(&i.section),
            });
            lw.globals.insert(key, globals.len());
            match ty {
                Ok(rt) => globals.push(ir::Slot::new(i.name.clone(), rt)),
                Err(e) => {
                    global_reasons.push(format!("dataitem `{}`: {e}", i.name));
                    globals.push(ir::Slot::new(i.name.clone(), RtType::Variant));
                }
            }
        }
        globals.push(ir::Slot::new("__userequestpage", RtType::Integer));
    }
    // A reportextension that changes a run names the base items' records first (the run points
    // them at the report's own), then the items it adds.
    if let (Some(base), Some(l)) = (seq.ext_base(oi), seq.ext(oi)) {
        let base_items = seq.report(base).map_or(&[][..], |b| &b.items[..]);
        for i in base_items.iter().chain(&l.items) {
            let key = i.name.to_lowercase();
            if lw.globals.contains_key(&key) {
                continue;
            }
            let ty = lw.rt(&Ty::Record {
                table: i.table.as_str().into(),
                temporary: crate::reports::uses_temporary(&i.section),
            });
            lw.globals.insert(key, globals.len());
            match ty {
                Ok(rt) => globals.push(ir::Slot::new(i.name.clone(), rt)),
                Err(e) => {
                    global_reasons.push(format!("dataitem `{}`: {e}", i.name));
                    globals.push(ir::Slot::new(i.name.clone(), RtType::Variant));
                }
            }
        }
    }
    for v in obj.var_decls() {
        let ty = v.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown);
        let label = v.type_ref().as_ref().and_then(label_text);
        let refusal = v.type_ref().as_ref().and_then(textconst_refusal);
        let attrs = var_attribute_names(&v);
        // A global record of a codeunit keeps its declared SecurityFiltering value (al-language/reports/runtime/ACAttributes.al:
        // 427-480), and so does one of a table, a page and a report, also through ClearAll after a SecurityFiltering(x) call
        // (al-language/reports/runtime/SFSecurityFilteringFlow.al:406-424). A global of any other object is not pinned.
        let declared = declared_security_filtering(&v, &attrs, || {
            matches!(
                o.kind,
                ObjKind::Codeunit | ObjKind::Table | ObjKind::Page | ObjKind::Report
            ) && matches!(lw.rt(&ty), Ok(RtType::Record(..)))
        });
        for n in v.names() {
            if !attrs.is_empty() {
                let mut left = attrs_left(&attrs, &declared);
                if with_events_without_trigger(o.kind, &ty, &obj, &n) {
                    for a in left.iter_mut().filter(|a| *a == "withevents") {
                        *a = WITH_EVENTS_NO_TRIGGER.into();
                    }
                }
                lw.global_var_attrs.insert(n.to_lowercase(), left);
            }
            if let Some(Err(e)) = &declared {
                global_reasons.push(format!("global `{n}`: {e}"));
            }
            note_global_label_facts(&mut lw, &v, &n, label.as_ref());
            if let Some(why) = refusal {
                global_reasons.push(format!("global `{n}`: {why}"));
            }
            if let Some(sig) = lw.tdd_object_sig(&ty) {
                lw.tdd_globals.insert(n.to_lowercase(), sig);
            }
            match lw.rt(&ty) {
                Ok(rt) => {
                    lw.globals.insert(n.to_lowercase(), globals.len());
                    let mut slot = ir::Slot::new(n, rt);
                    slot.security_filtering = declared.clone().and_then(Result::ok);
                    slot.security_filtering_not_kept = security_filtering_not_kept(&attrs, &declared);
                    globals.push(slot);
                }
                Err(e) => global_reasons.push(format!("global `{n}`: {e}")),
            }
        }
    }
    // The variables of the request page (`var` in a `requestpage` section) are slots of their own, seen by the
    // triggers of that request page only (corpus reports/request-page/ABRequestPageExt.al:63).
    for v in obj.request_page_var_decls() {
        let ty = v.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown);
        for n in v.names() {
            match lw.rt(&ty) {
                Ok(rt) => {
                    lw.request_page_globals.insert(n.to_lowercase(), globals.len());
                    globals.push(ir::Slot::new(format!("requestpage.{n}"), rt));
                }
                Err(e) => global_reasons.push(format!("request page variable `{n}`: {e}")),
            }
        }
    }
    let (mut rec_slot, mut xrec_slot) = (None, None);
    if let Some(t) = table_of[oi] {
        for (name, slot) in [("Rec", &mut rec_slot), ("xRec", &mut xrec_slot)] {
            // A codeunit has no xRec.
            if name == "xRec" && o.kind == ObjKind::Codeunit {
                continue;
            }
            *slot = Some(globals.len());
            lw.globals.insert(name.to_lowercase(), globals.len());
            globals.push(ir::Slot::new(
                name,
                RtType::Record(t as u32, lw.is_temporary_table(t) || page_temporary[oi]),
            ));
        }
    }
    // A query's own code runs on the query it was called on: its state sits in a hidden slot
    // the VM fills when it makes the instance (`Session::query_instance`).
    if o.kind == ObjKind::Query {
        rec_slot = Some(globals.len());
        // `CurrQuery` in the query's triggers and procedures
        // (queries/TestQueryOnBeforeOpenVariablesContracts.al:75-77).
        lw.globals.insert("currquery".into(), globals.len());
        globals.push(ir::Slot::new("__query", RtType::Query(oi as u32)));
    }
    lw.rec_slot = rec_slot;
    if o.kind == ObjKind::Report {
        // `CurrPage` of the request page. Typed Variant: the test client puts the page there
        // when it opens, and AL code cannot name it.
        globals.push(ir::Slot::new("CurrPage", RtType::Variant));
    }
    // The page an extension extends, when it is in the workspace.
    let extended_page = (o.kind == ObjKind::PageExtension)
        .then(|| page_extensions.iter().find(|(_, v)| v.contains(&oi)).map(|(&p, _)| p))
        .flatten();
    if o.kind == ObjKind::Page || extended_page.is_some() {
        lw.globals.insert("currpage".into(), globals.len());
        globals.push(ir::Slot::new(
            "CurrPage",
            RtType::TestPage(extended_page.unwrap_or(oi) as u32),
        ));
    }
    if extended_page.is_some() {
        debug_assert!(
            sh.ext_slots[oi]
                .iter()
                .all(|(n, &i)| globals.get(i).is_some_and(|g| g.name.eq_ignore_ascii_case(n))),
            "the slots of a page extension's variables moved"
        );
    }
    let mut routines = Vec::new();
    let mut lowered_routines = 0;
    let mut unsupported = Vec::new();
    let mut refusal_at = Vec::new();
    let mut calls = Vec::new();
    let mut direct = Vec::new();
    let mut ndirect = Vec::new();
    for (ri, r) in tl.nodes.get(oi).iter().enumerate() {
        lw.routine_idx = ri;
        lw.in_request_page = matches!(o.kind, ObjKind::Report | ObjKind::ReportExtension)
            && r.syntax()
                .ancestors()
                .filter_map(ast::Section::cast)
                .any(|s| s.name().eq_ignore_ascii_case("requestpage"));
        lw.in_report_init_with_rec = (o.kind == ObjKind::Report
            && table_of[oi].is_some()
            && r.is_trigger()
            && r.name().eq_ignore_ascii_case("OnInitReport")
            && r.syntax().ancestors().filter_map(ast::Section::cast).next().is_none())
        .then(|| o.id.map(i64::from).unwrap_or_default());
        lw.reasons = global_reasons.clone();
        lw.refusal_at = None;
        lw.calls.clear();
        // A trigger of a data item runs with the item's record as its `Rec`.
        if matches!(o.kind, ObjKind::Report | ObjKind::ReportExtension) {
            lw.rec_slot = r
                .syntax()
                .ancestors()
                .filter_map(ast::Section::cast)
                .find(|s| {
                    ["dataitem", "modify", "add"]
                        .iter()
                        .any(|k| s.name().eq_ignore_ascii_case(k))
                })
                .and_then(|s| s.args().into_iter().next())
                .and_then(|n| lw.globals.get(&n.to_lowercase()).copied());
            // A trigger of the request page runs with the page's `Rec`.
            if lw.rec_slot.is_none()
                && o.kind == ObjKind::Report
                && r.syntax()
                    .ancestors()
                    .filter_map(ast::Section::cast)
                    .any(|s| s.name().eq_ignore_ascii_case("requestpage"))
            {
                lw.rec_slot = rec_slot;
            }
        }
        // A dependency routine nothing reached was never checked: a stub that refuses.
        let reached = !checked.is_skipped(oi)
            && !ws.routine_unreached(oi, routine_offsets[oi][ri])
            && demand.is_none_or(|d| !ws.is_dependency(oi) || d.reached(oi, routine_offsets[oi][ri]));
        // A routine whose check panicked has no types and resolutions to lower from: it refuses.
        let routine = if let Some(why) = checked.panic_of(oi, routine_offsets[oi][ri]) {
            lw.refused(r, format!("internal error while checking: {why}"))
        } else if reached {
            lowered_routines += 1;
            // A panic lowering one routine is a bug in the compiler; the routine refuses and the
            // run goes on.
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lw.routine(r))) {
                Ok(routine) => routine,
                Err(p) => {
                    let p = crate::halt::rethrow_halt(p);
                    lw.calls.clear();
                    lw.reasons = global_reasons.clone();
                    lw.refused(
                        r,
                        format!("internal error while lowering: {}", crate::check::panic_message(&*p)),
                    )
                }
            }
        } else if checked.is_left_out(oi) {
            lw.refused(r, crate::check::LEFT_OUT.into())
        } else if !ws.is_dependency(oi) {
            // A workspace routine is compiled unless the selection left it out: a test codeunit it does not keep, or a
            // procedure no selected test names (`Roots::reach_only`).
            lw.refused(r, crate::check::UNSELECTED.into())
        } else {
            lw.stub(r)
        };
        // An action's OnAction that cannot be lowered is refused when the action runs: a test that opens the page and never
        // invokes the action runs (Base Application's Item Card has Xmlport.Run and Media.ExportFile in actions; 81
        // Tests-SCM tests only open the card). Only a routine that was compiled and checked: a stub keeps its static reason.
        let mut routine = routine;
        if reached
            && checked.panic_of(oi, routine_offsets[oi][ri]).is_none()
            && matches!(o.kind, ObjKind::Page | ObjKind::PageExtension)
            && r.is_trigger()
            && r.name().eq_ignore_ascii_case("onaction")
            && r.syntax()
                .ancestors()
                .filter_map(ast::Section::cast)
                .any(|s| s.name().eq_ignore_ascii_case("action"))
            && lw.reasons.len() > global_reasons.len()
        {
            let why = lw.reasons[global_reasons.len()..].join("; ");
            routine.body = vec![ir::Stmt::Expr(ir::Expr::Refused(format!("{}.OnAction: {why}", o.name)))].into();
            lw.reasons = global_reasons.clone();
            lw.calls.clear();
        }
        if sh.export {
            direct.push(lw.calls.clone());
        }
        // A TestPage drives page routines the call graph cannot see (ADR-012). An xmlport's
        // triggers are run by Import and Export, and a report's by Run, which the graph cannot
        // see either.
        ndirect.push(lw.calls.len() as u32);
        driven_edges(&sh.driven, routine.locals.iter().chain(&globals), &mut lw.calls);
        unsupported.push(std::mem::take(&mut lw.reasons).into_iter().map(Refusal::from).collect());
        refusal_at.push(lw.refusal_at.take());
        calls.push(std::mem::take(&mut lw.calls));
        routines.push(routine);
    }
    let request_section = (o.kind == ObjKind::Report)
        .then(|| obj.sections().find(|s| s.name().eq_ignore_ascii_case("requestpage")))
        .flatten();
    let page = seq
        .page_layout(oi)
        .map(|l| lw.page(oi, (&obj, request_section.as_ref()), l, &globals, &page_reasons[oi]));
    // A report's request page properties (`SaveValues`, `Caption`) read like a page's: the
    // report's own `Caption` wins over the request page's
    // (corpus reports/TestReportRequestPageCaption.al).
    let mut properties = with_profile_target(ws, oi, object_properties(&obj, request_section.as_ref()));
    let handler_props = super::test_handlers::properties(ws, members, oi, &properties);
    properties.extend(handler_props);
    ext_caption_properties(
        ws,
        extensions.get(&oi).map(Vec::as_slice).unwrap_or_default(),
        &o.name,
        &mut properties,
    );
    let (table, key_owners) = if o.kind == ObjKind::Table && ws.usable(oi) {
        let (t, owners) = lw.table_schema(oi, &obj, extensions.get(&oi).cloned().unwrap_or_default(), routine_at);
        (Some(t), owners)
    } else {
        (None, Vec::new())
    };
    let object = ir::Object {
        kind: o.kind,
        uncompiled: false,
        id: o.id,
        name: o.name.clone(),
        globals,
        routines,
        properties,
        enum_values: enum_pairs(&members[oi].enum_values),
        enum_captions: enum_pairs(&members[oi].enum_captions),
        enum_captions_ml: enum_pairs(&members[oi].enum_captions_ml),
        // A table whose source cannot be used (a platform table nothing provides, a dependency
        // file that does not parse) has no members, so it has no schema: the Field and Key
        // tables report it as not compiled instead of reading fields it does not have.
        table,
        rec_slot,
        xrec_slot,
        page,
        app: o.app as u32,
        xmlport: xml_irs[oi].clone(),
        report: report_irs[oi].clone(),
        report_ext: ext_irs[oi].clone(),
        namespace: o.namespace.clone(),
        extends_id: ws.extension_target(oi).and_then(|t| ws.objects[t].id),
        layouts: if matches!(o.kind, ObjKind::Report | ObjKind::ReportExtension) {
            crate::reports::layouts_of(&obj)
        } else {
            Vec::new()
        },
        query: checked.queries[oi].as_ref().map(|q| q.query.clone()),
    };
    let positions = std::mem::take(&mut lw.positions);
    // Itself, whatever the lowering read about the extensions and implementers of others, and
    // what `provided::embeds_of` adds from the pages' parts.
    let embeds = lw.embeds.take().into_iter().collect();
    Piece {
        object,
        lowered_routines,
        positions,
        unsupported,
        refusal_at,
        calls,
        direct,
        ndirect,
        embeds,
        key: None,
        field_refs: lw.field_refs.take().list,
        control_refs: lw.control_refs.take().list,
        expansions: lw.expansions.take().list,
        page_mods: lw.page_mods.take(),
        sym: None,
        key_owners,
    }
}

/// A tableextension's Caption is the caption of a table that declares none (table-objects/extensions/TestTableExtFieldModify.al:
/// 168-178: TableCaption, RecordRef.Caption and AllObjWithCaption of the table). With a Caption on the table too, or on
/// two extensions, which one wins is not pinned: the reads of the caption refuse (`CAPTION_REFUSED`). One extension's Caption
/// over the table's own is the caption the runtime reads (`EXT_CAPTION_OVER_OWN`; corpus writer Y5, branch
/// al-runtime-rs-staging-y5, recordref-fieldref/Y5TestFieldExtCaption.al); AllObjWithCaption of it stays refused.
fn ext_caption_properties(ws: &Workspace, exts: &[usize], name: &str, properties: &mut Vec<(String, String)>) {
    if exts.is_empty() {
        return;
    }
    {
        let ext_captions: Vec<(String, String)> = exts
            .iter()
            .flat_map(|&e| {
                ws.object_ast(e)
                    .properties()
                    .filter(|p| p.name().eq_ignore_ascii_case("Caption") || p.name().eq_ignore_ascii_case("CaptionML"))
                    .map(move |p| (p.name(), p.value_text()))
                    .collect::<Vec<_>>()
            })
            .collect();
        let own = properties
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Caption") || k.eq_ignore_ascii_case("CaptionML"));
        match ext_captions.as_slice() {
            [] => {}
            [(k, v)] if !own && k.eq_ignore_ascii_case("Caption") => properties.push((k.clone(), v.clone())),
            _ => {
                if let [(k, v)] = ext_captions.as_slice() {
                    if k.eq_ignore_ascii_case("Caption") {
                        properties.push((ir::EXT_CAPTION_OVER_OWN.into(), v.clone()));
                    }
                }
                properties.push((
                ir::CAPTION_REFUSED.into(),
                format!(
                    "the caption of table {}, which a tableextension sets {} (which caption BC shows is not pinned)",
                    name,
                    if own {
                        "over its own"
                    } else {
                        "with CaptionML or more than once"
                    }
                ),
            ))
            }
        }
    }
}

/// The page triggers an extension can declare that the client runs after the page's own trigger.
/// The others answer a Boolean or serve rows, and how an extension's answer combines with the
/// page's is not pinned. OnInit, OnFindRecord and OnNextRecord are page-only in BC: an extension
/// that declares one is not called for it (AL Runner v2 docs/../RecordPatches.PageTriggerMetadata.cs,
/// BC 27.5 and 28.4 NCLMetaForm.DefinedTriggers), so they are refused here.
pub(crate) const EXTENSION_PAGE_TRIGGERS: &[&str] = &[
    "onopenpage",
    "onclosepage",
    "onaftergetrecord",
    "onaftergetcurrrecord",
    "onnewrecord",
    // Both the page's and the extensions' answers count, and both must be true to close
    // (testpage/TestPageCloseTriggersSelectionCardPageContracts.al:
    // OnQueryClosePage_PageAndExtension_BothRun_AndBothMustAgreeToClose).
    "onqueryclosepage",
    // The test client has no delete operation, and a page action's `Rec.Delete(true)` runs neither the page's nor the
    // extension's trigger (pages/extensions/PageExtOnDeleteRecord.al:108-193, all four result
    // combinations). A client delete is not available, so the trigger never runs here.
    "ondeleterecord",
    // All of them run, the page's first, and an extension's false answer vetoes the insert or the modify (pageext/
    // lt-ue-pageext-leads/UeTriggerTests.al, al-runner issue 6149; assumption pageext-record-trigger-answers until the pin lands).
    "oninsertrecord",
    "onmodifyrecord",
    // The page's trigger runs first, then the extension's, for every task, whoever enqueued it; the error trigger hands its
    // IsHandled on to the next one (collections/UjPageExtBgTask.al, corpus branch al-runtime-rs-staging-uj
    // 1dc037a8, al-runner issue 6182; assumption pageext-background-task-triggers until the pin lands).
    "onpagebackgroundtaskcompleted",
    "onpagebackgroundtaskerror",
];

/// The triggers a `modify` block can add to a control, which the client runs.
pub(crate) const MODIFY_TRIGGERS: &[&str] = &[
    "onbeforevalidate",
    "onaftervalidate",
    "ondrilldown",
    "onlookup",
    "onassistedit",
];

/// Why the client cannot drive a page once the extensions `exts` are merged into it: the
/// triggers it does not run. Several extensions may declare one trigger: they run in the order
/// the extensions apply (docs/probes-needed.md, "Page extensions").
pub(crate) fn extension_reasons(exts: &[(usize, ast::Object)]) -> Vec<String> {
    let mut out = Vec::new();
    for (_, e) in exts {
        let ename = e.name();
        for t in e.triggers() {
            if !EXTENSION_PAGE_TRIGGERS.contains(&t.name().to_ascii_lowercase().as_str()) {
                out.push(format!(
                    "page extension `{ename}`: trigger {} is not supported yet",
                    t.name()
                ));
            }
        }
        // Triggers inside `modify` blocks: only the ones the client dispatches.
        for sec in ast::sections_in(e.syntax()) {
            if !sec.name().eq_ignore_ascii_case("modify") {
                continue;
            }
            for t in sec.triggers() {
                // OnAfterAction of a modified action runs after its OnAction, OnBeforeAction before it (pages/extensions/
                // PageExtensionActionOnAfterAction.al; OnBeforeAction: testpage/ABTests.al:979-1018 and UQ_* on the corpus
                // branch al-runtime-rs-staging-uq at d651ec93; the VM refuses the shapes that file does not pin when the action is invoked).
                let name = t.name().to_ascii_lowercase();
                if !MODIFY_TRIGGERS.contains(&name.as_str()) && name != "onafteraction" && name != "onbeforeaction" {
                    out.push(format!(
                        "page extension `{ename}`: trigger {} in modify({}) is not supported yet",
                        t.name(),
                        sec.args().first().cloned().unwrap_or_default()
                    ));
                }
            }
        }
    }
    out
}

/// An `InitValue` property as a constant of the field's type; `None` for
/// forms not handled yet (the field then starts at its type's default).
pub(crate) fn init_const(text: &str, ty: &Ty) -> Option<ir::Const> {
    let t = text.trim();
    Some(match ty {
        Ty::Integer => ir::Const::Integer(t.parse().ok()?),
        Ty::BigInteger => ir::Const::BigInteger(t.parse().ok()?),
        Ty::Decimal => ir::Const::Decimal(t.parse::<rust_decimal::Decimal>().ok()?.to_string()),
        Ty::Boolean if t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes") => ir::Const::Boolean(true),
        Ty::Boolean if t.eq_ignore_ascii_case("false") || t.eq_ignore_ascii_case("no") => ir::Const::Boolean(false),
        Ty::Text(_) | Ty::Code(_) => ir::Const::Text(
            t.strip_prefix('\'')
                .and_then(|x| x.strip_suffix('\''))
                .unwrap_or(t)
                .replace("''", "'"),
        ),
        Ty::Option(m) => {
            // Both sides trimmed, as for an enum: the blank member `" "` is written `const(" ")` in a TableRelation
            // (Finance Charge Memo Line."No."; unprobed for an Option, the enum form is pinned by Standard Service Line."No.").
            let name = t.trim_matches('"').trim();
            ir::Const::Option(
                m.iter().position(|x| x.trim().eq_ignore_ascii_case(name))? as i32,
                m.clone(),
            )
        }
        // The text of a Guid reads as `Evaluate` reads it (`parse_guid`); a `const('..')` of a CalcFormula on a Guid field.
        Ty::Guid => ir::Const::Guid(al_builtins::ops::parse_guid(
            t.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')).unwrap_or(t),
        )?),
        Ty::Date => ir::Const::Date(date_literal(t)?),
        Ty::Time => ir::Const::Time(time_literal(t)?),
        // The same reading as `Evaluate` into a DateFormula (types/TestBlankEnumInitTests.Codeunit.al:88-95
        // says InitValue and Evaluate share one evaluator): a calculable formula is kept as written.
        // Blank, garbled and not-yet-supported formulas stay refused (the field is not initialised).
        Ty::DateFormula => {
            let f = t.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')).unwrap_or(t);
            if al_builtins::datefns::is_blank_formula(f)
                || al_builtins::datefns::is_garbage_formula(f)
                || al_builtins::datefns::check_formula(f).is_err()
            {
                return None;
            }
            ir::Const::DateFormula(f.to_string())
        }
        _ => return None,
    })
}

/// The value `raw` of a filter or const condition on field `field` of type `ty`, with a member qualifier dropped: the field's enum
/// or the field's own name on an enum field, the field's name on an Option field (`al_builtins::enum_qualifier::strip_enum_qualifier`
/// holds the pins). Any other value as it is.
pub(crate) fn unqualified_member(raw: &str, ty: &Ty, field: &str) -> String {
    use al_builtins::enum_qualifier::strip_enum_qualifier;
    match ty {
        Ty::Enum(name) => strip_enum_qualifier(raw, &[&crate::types::display_name(name), field]).into_owned(),
        Ty::Option(_) => strip_enum_qualifier(raw, &[field]).into_owned(),
        _ => raw.to_string(),
    }
}

/// A `const(..)` value as a constant of the source field's type.
pub(crate) fn calc_const(
    ws: &Workspace,
    members: &MemberTable<'_>,
    file: usize,
    raw: &str,
    ty: &Ty,
) -> Result<ir::Const, String> {
    let raw = raw.trim();
    // `Database::"X"` / `Report::"X"`: the object's id (corpus
    // record/TestCalcFormulaDatabaseConst.Codeunit.al:7).
    if let Some((kind, name)) = raw.split_once("::") {
        let kind = match kind.trim().to_ascii_lowercase().as_str() {
            "database" => Some(ObjKind::Table),
            other => ObjKind::from_type_name(other),
        };
        let name = name.trim().trim_matches('"').to_string();
        let Some(kind) = kind.filter(|k| *k != ObjKind::Enum) else {
            return Err(format!("CalcFormula const({raw}) is not understood yet"));
        };
        let Resolution::Found(o) = ws.resolve_ref(file, kind, std::slice::from_ref(&name)) else {
            return Err(format!("CalcFormula const({raw}): `{name}` is outside the workspace"));
        };
        let id = ws.objects[o]
            .id
            .ok_or_else(|| format!("CalcFormula const({raw}) has no object id"))?;
        return match ty {
            Ty::Integer => Ok(ir::Const::Integer(id as i32)),
            Ty::BigInteger => Ok(ir::Const::BigInteger(i64::from(id))),
            Ty::Decimal => Ok(ir::Const::Decimal(id.to_string())),
            _ => Err(format!("CalcFormula const({raw}) on a non-numeric field")),
        };
    }
    let unquoted = raw.trim_matches('"');
    if let Ty::Enum(name) = ty {
        let Resolution::Found(e) = ws.resolve_ref(file, ObjKind::Enum, &crate::types::qualified_parts(name)) else {
            return Err(format!(
                "CalcFormula const({raw}): enum `{}` is outside the workspace",
                crate::types::display_name(name)
            ));
        };
        let values = &members[e].enum_values;
        let ordinal = values
            .iter()
            .find(|(_, n)| n.eq_ignore_ascii_case(unquoted))
            .map(|(v, _)| *v as i32)
            .or_else(|| unquoted.parse().ok())
            .ok_or_else(|| format!("CalcFormula const({raw}) is not a member of enum `{name}`"))?;
        return Ok(ir::Const::Enum(e as u32, ordinal));
    }
    if let Ty::Option(members) = ty {
        if let Ok(n) = unquoted.parse::<i32>() {
            return Ok(ir::Const::Option(n, members.clone()));
        }
    }
    init_const(raw, ty).ok_or_else(|| format!("CalcFormula const({raw}) is not understood for a {ty} field"))
}

/// The runtime type of a scalar type (numbers, text, dates, options, enums); `None` for the rest.
pub(crate) fn scalar_rt(ws: &Workspace, file: usize, t: &Ty) -> Option<L<RtType>> {
    Some(Ok(match t {
        Ty::Integer => RtType::Integer,
        Ty::BigInteger => RtType::BigInteger,
        Ty::Decimal => RtType::Decimal,
        Ty::Boolean => RtType::Boolean,
        Ty::Char => RtType::Char,
        Ty::Byte => RtType::Byte,
        Ty::Text(n) => RtType::Text(*n),
        Ty::Code(n) => RtType::Code(*n),
        Ty::Label => RtType::Text(None),
        Ty::Date => RtType::Date,
        Ty::Time => RtType::Time,
        Ty::DateTime => RtType::DateTime,
        Ty::Duration => RtType::Duration,
        Ty::DateFormula => RtType::DateFormula,
        Ty::Guid => RtType::Guid,
        Ty::Variant => RtType::Variant,
        Ty::Option(m) => RtType::Option(m.as_slice().into()),
        Ty::Enum(n) => match ws.resolve_ref(file, ObjKind::Enum, &crate::types::qualified_parts(n)) {
            Resolution::Found(i) => RtType::Enum(i as u32),
            _ => {
                return Some(Err(format!(
                    "{} `{}` is outside the workspace",
                    ObjKind::Enum.label(),
                    crate::types::display_name(n)
                )));
            }
        },
        _ => return None,
    }))
}

/// `20240115D` / `0D` as a day number.
pub(crate) fn date_literal(text: &str) -> Option<i32> {
    let d = text.strip_suffix(['D', 'd'])?;
    if d.chars().all(|c| c == '0') {
        return Some(0);
    }
    if d.len() != 8 || !d.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(al_builtins::format::day_from_civil(
        d[..4].parse().ok()?,
        d[4..6].parse().ok()?,
        d[6..].parse().ok()?,
    ))
}

/// `120000T` / `120000.5T` / `0T` as milliseconds since midnight plus one.
pub(crate) fn time_literal(text: &str) -> Option<i64> {
    let d = text.strip_suffix(['T', 't'])?;
    if d.chars().all(|c| c == '0') && !d.is_empty() && d.len() < 6 {
        return Some(0);
    }
    let (hms, frac) = d.split_once('.').unwrap_or((d, "0"));
    if hms.len() != 6 || !hms.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let h: i64 = hms[..2].parse().ok()?;
    let m: i64 = hms[2..4].parse().ok()?;
    let s: i64 = hms[4..].parse().ok()?;
    let ms: i64 = format!("{frac:0<3}")[..3].parse().ok()?;
    Some(((h * 60 + m) * 60 + s) * 1000 + ms + 1)
}

/// The text of a `Label '...'` declaration, quotes undoubled; of a `TextConst ENU = '...', DEU = '...'` declaration, the
/// text of the language `ENU` (types/variant/VaTTextConst.al:TextConst_To_Text_Basics: `multi` with DEU, ENU and FRA reads
/// as the ENU text), else its first text.
pub(crate) fn label_text(t: &ast::TypeRef) -> Option<String> {
    let unquote = |text: &str| text[1..text.len() - 1].replace("''", "'");
    let kind = t.type_name();
    if kind.eq_ignore_ascii_case("label") {
        let tok = t
            .syntax()
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|k| k.kind() == SyntaxKind::StringLit)?;
        let text = unquote(tok.text());
        // `MaxLength = n` cuts the text to n characters (text/textbuilder-bigtext/TxCLabel.al:Label_MaxLength).
        let toks: Vec<_> = t
            .syntax()
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|k| !k.kind().is_trivia())
            .collect();
        let max = toks.windows(3).find_map(|w| {
            (w[0].text().eq_ignore_ascii_case("maxlength") && w[1].kind() == SyntaxKind::Eq)
                .then(|| w[2].text().parse::<usize>().ok())
                .flatten()
        });
        return Some(match max {
            Some(n) => text.chars().take(n).collect(),
            None => text,
        });
    }
    if !kind.eq_ignore_ascii_case("textconst") {
        return None;
    }
    // The ENU text, else the first text: the read of a TextConst whose text depends on the language in force goes through
    // [`textconst_languages`] instead.
    let texts = textconst_entries(t);
    let pick = texts.iter().find(|(lang, _)| lang == "ENU").or(texts.first())?;
    Some(pick.1.clone())
}

/// The language codes (uppercase) and texts of a `TextConst` declaration, in declaration order.
fn textconst_entries(t: &ast::TypeRef) -> Vec<(String, String)> {
    let unquote = |text: &str| text[1..text.len() - 1].replace("''", "'");
    let toks: Vec<_> = t
        .syntax()
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|k| !k.kind().is_trivia())
        .collect();
    toks.windows(3)
        .filter(|w| {
            w[0].kind() == SyntaxKind::Ident && w[1].kind() == SyntaxKind::Eq && w[2].kind() == SyntaxKind::StringLit
        })
        .map(|w| (w[0].text().to_ascii_uppercase(), unquote(w[2].text())))
        .collect()
}

/// The texts of a `TextConst` whose read depends on the language in force (several languages, or none of them ENU), by
/// language code in declaration order; `None` for a Label or a TextConst with only an ENU text. A read lowers to
/// `System.TextConstText#1`, which picks the text when it runs (text/textconst-label/P140TextConstLang.al,
/// P140TextConstNoEnu.al).
pub(crate) fn textconst_languages(t: &ast::TypeRef) -> Option<Vec<(String, String)>> {
    if !t.type_name().eq_ignore_ascii_case("textconst") {
        return None;
    }
    let texts = textconst_entries(t);
    (!texts.is_empty() && (texts.len() > 1 || texts[0].0 != "ENU")).then_some(texts)
}

/// Whether a `Label` declaration says `Locked = true`: the compiler writes no translation unit for it, so no translation file
/// changes its text (ADR-053 decision 2).
pub(crate) fn label_locked(t: &ast::TypeRef) -> bool {
    let toks: Vec<_> = t
        .syntax()
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|k| !k.kind().is_trivia())
        .collect();
    toks.windows(3).any(|w| {
        w[0].text().eq_ignore_ascii_case("locked")
            && w[1].kind() == SyntaxKind::Eq
            && w[2].text().eq_ignore_ascii_case("true")
    })
}

/// Whether a Label declaration sets `MaxLength`.
pub(crate) fn label_maxlength(t: &ast::TypeRef) -> bool {
    let toks: Vec<_> = t
        .syntax()
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|k| !k.kind().is_trivia())
        .collect();
    toks.windows(2)
        .any(|w| w[0].text().eq_ignore_ascii_case("maxlength") && w[1].kind() == SyntaxKind::Eq)
}

/// Whether a declaration is of type `TextConst`.
pub(crate) fn is_textconst(t: Option<&ast::TypeRef>) -> bool {
    t.is_some_and(|t| t.type_name().eq_ignore_ascii_case("textconst"))
}

/// The lowering reason of a declaration that cannot be read: a `TextConst` with no text in any language.
pub(crate) fn textconst_refusal(t: &ast::TypeRef) -> Option<&'static str> {
    (t.type_name().eq_ignore_ascii_case("textconst") && label_text(t).is_none())
        .then_some("TextConst without a text (not pinned)")
}

/// A filter expression written in AL, with double-quoted names turned into
/// the single-quoted literals the filter grammar understands. AL quotes an
/// identifier with `"`; the filter grammar treats `"` as an ordinary
/// character (corpus record/TestCalcFormulaQuotedFilterTests.Codeunit.al:9).
pub(crate) fn rewrite_quotes(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push('\'');
                for n in chars.by_ref() {
                    match n {
                        '"' => break,
                        '\'' => out.push_str("''"),
                        n => out.push(n),
                    }
                }
                out.push('\'');
            }
            '\'' => {
                out.push('\'');
                while let Some(n) = chars.next() {
                    out.push(n);
                    if n == '\'' {
                        if chars.peek() == Some(&'\'') {
                            out.push(chars.next().expect("peeked"));
                        } else {
                            break;
                        }
                    }
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// Whether the routines of dependency apps carry source positions (ADR-041). On unless `AL_NO_DEP_POSITIONS` is set.
pub fn dependency_positions() -> bool {
    std::env::var_os("AL_NO_DEP_POSITIONS").is_none()
}

/// The lower-case names of the attributes of a variable declaration that change how the variable behaves and that
/// the lowering reads (`Lowerer::var_attr_refusal`, `Lowerer::security_filtering_read_refusal`).
pub(crate) fn var_attribute_names(v: &ast::VarDecl) -> Vec<String> {
    v.attributes()
        .iter()
        .map(|a| a.name().to_ascii_lowercase())
        .filter(|n| matches!(n.as_str(), "runonclient" | "withevents" | "securityfiltering"))
        .collect()
}

/// The SecurityFiltering value `[SecurityFiltering(SecurityFilter::X)]` declares (the member index of X), for a variable
/// whose `attrs` ([`var_attribute_names`]) name it and that `keeps` it (a record variable the runtime keeps the value of:
/// al-ir `Slot::security_filtering`). `None` otherwise: the read of the value then stays refused
/// (`Lowerer::security_filtering_read_refusal`). An error for an argument that names no member of SecurityFilter.
pub(crate) fn declared_security_filtering(
    v: &ast::VarDecl,
    attrs: &[String],
    keeps: impl FnOnce() -> bool,
) -> Option<Result<i32, String>> {
    if !attrs.iter().any(|a| a == "securityfiltering") || !keeps() {
        return None;
    }
    use al_syntax::ast::NodeText;
    let attr = v
        .attributes()
        .into_iter()
        .find(|a| a.name().eq_ignore_ascii_case("SecurityFiltering"))?;
    let text = attr.args().map(|a| a.text_string()).unwrap_or_default();
    let inner = text.trim().trim_start_matches('(').trim_end_matches(')').trim();
    let member = inner.rsplit("::").next().unwrap_or("").trim().trim_matches('"');
    let members = al_builtins::sysenum::option_members("SecurityFilter").expect("known");
    Some(
        members
            .iter()
            .position(|m| m.eq_ignore_ascii_case(member))
            .map(|i| i as i32)
            .ok_or_else(|| format!("the attribute SecurityFiltering({inner}) is not understood")),
    )
}

/// The attributes the lowering still reads for a variable with `attrs`: SecurityFiltering becomes [`SECURITY_FILTERING_KEPT`]
/// when its value is kept.
pub(crate) fn attrs_left(attrs: &[String], declared: &Option<Result<i32, String>>) -> Vec<String> {
    let mut attrs = attrs.to_vec();
    if declared.is_some() {
        for a in attrs.iter_mut().filter(|a| *a == "securityfiltering") {
            *a = SECURITY_FILTERING_KEPT.into();
        }
    }
    attrs
}

/// Whether the global `name` of the object `obj` is a `[WithEvents]` DotNet variable of a codeunit that declares no
/// `trigger name::Event` for it. No AL code then runs on its events, and its uses run as a plain DotNet variable's, marked
/// with assumption `with-events-no-trigger` (Microsoft's Sales and Inventory Forecast Tests, codeunit 139540
/// TestMissingApiUriOpenSetup, passes on real BC through Base App's Azure ML Connector, whose WithEvents variables have no
/// trigger). Every other WithEvents variable keeps the refusal (`Lowerer::var_attr_refusal`).
fn with_events_without_trigger(kind: ObjKind, ty: &Ty, obj: &ast::Object, name: &str) -> bool {
    let prefix = format!("{}::", name.to_lowercase());
    kind == ObjKind::Codeunit
        && matches!(ty, Ty::DotNet(_))
        && !obj.triggers().any(|t| t.name().to_lowercase().starts_with(&prefix))
}

/// The name a `[WithEvents]` attribute gets for a variable [`with_events_without_trigger`] accepts.
pub(crate) const WITH_EVENTS_NO_TRIGGER: &str = "withevents-no-trigger";

/// The name [`attrs_left`] gives the attribute SecurityFiltering of a variable whose declared value the runtime keeps: its
/// read is not refused, an array assignment or CopyArray of it is (`Lowerer::security_filtering_array_refusal`).
pub(crate) const SECURITY_FILTERING_KEPT: &str = "securityfiltering-kept";

/// Whether a variable with the attribute names `attrs` ([`var_attribute_names`]) was declared with SecurityFiltering, but
/// the runtime does not keep its declared value (al-ir `Slot::security_filtering_not_kept`).
pub(crate) fn security_filtering_not_kept(attrs: &[String], declared: &Option<Result<i32, String>>) -> bool {
    declared.is_none() && attrs.iter().any(|a| a == "securityfiltering")
}

/// [`over_max_length`] of object `oi`, without a walk of its properties when its file names no `MaxLength` at all (nearly
/// every file): only a `MaxLength` attribute makes a text over its MaxLength, and the file's text holds the object's.
pub(crate) fn over_max_length_of(ws: &crate::workspace::Workspace, oi: usize) -> Vec<String> {
    if !ws.names_max_length(ws.objects[oi].file) {
        return Vec::new();
    }
    over_max_length(&ws.object_ast(oi))
}

/// The text properties of `obj` (its controls, fields, columns and actions included) whose text is longer than their
/// `MaxLength`, as refusal reasons. Only a `Caption` and a Label are pinned (they are cut: record/table-properties/
/// UTableTextSearch.al:41-44, text/textbuilder-bigtext/TxCLabel.al:282); on any other property whether BC cuts the text is
/// not pinned, so the object that holds it is refused where it is used.
pub(crate) fn over_max_length(obj: &ast::Object) -> Vec<String> {
    obj.syntax()
        .descendants()
        .filter_map(ast::Property::cast)
        .filter(|p| al_syntax::ast::text_over_max_length(&p.value_text()))
        .map(|p| {
            format!(
                "property {} of {} has a text longer than its MaxLength (whether it is cut is not pinned)",
                p.name(),
                obj.name()
            )
        })
        .collect()
}

/// An object's properties, and for a report its request page's ones (see `lower_object`).
/// A `profileextension` is an object of kind `Other`; the property [`ir::EXTENDS_PROFILE`] names the profile it extends.
fn with_profile_target(ws: &Workspace, oi: usize, mut properties: Vec<(String, String)>) -> Vec<(String, String)> {
    let o = &ws.objects[oi];
    if let (ObjKind::Other, Some(name)) = (o.kind, o.extends.as_ref().and_then(|e| e.last())) {
        if ws.object_ast(oi).kind() == "profileextension" {
            properties.push((ir::EXTENDS_PROFILE.to_string(), name.clone()));
        }
    }
    properties
}

fn object_properties(obj: &ast::Object, request_section: Option<&ast::Section>) -> Vec<(String, String)> {
    let mut properties: Vec<(String, String)> = obj.properties().map(|p| (p.name(), p.value_text())).collect();
    if let Some(sec) = request_section {
        for p in sec.properties() {
            let name = p.name();
            if name.eq_ignore_ascii_case("Caption") {
                properties.push((ir::REQUEST_PAGE_CAPTION_PROPERTY.to_string(), p.value_text()));
            }
            if name.eq_ignore_ascii_case("Caption") && properties.iter().any(|(k, _)| k.eq_ignore_ascii_case("Caption"))
            {
                continue;
            }
            properties.push((name, p.value_text()));
        }
    }
    properties
}

/// Why a restricted session (permission sets a test lowered) refuses to open the page of `obj`. A field, action or part of the page's
/// own layout with an AccessByPermission property is decided when it is used (Y24PermExecute.al Y24_P21_*, Y84PermAbp.al: BC leaves
/// it off the page when the sets do not grant the permission; `Session::access_by_permission_hidden_decided` reads the guards). So
/// is a control over a table field that carries the property (the control takes the field's guard, see `inherited`), and the
/// property on the page or report object itself, which has no effect on a run (Y84_Abp_PageAndReportLevel). An actionref to a guarded
/// action (in area(Promoted) or any other area, in a group, added by a page extension) changes nothing a test sees: the action is
/// decided by its own guard (Y90PermActionRef.al:31-53, Y94AbpActionRef.al:23-110 for every guard kind, Y94AbpList.al:19-41,
/// Y94AbpPageLevel.al), and a TestPage call on the actionref itself is decided at run time (`Session::access_by_permission_check`).
/// Any other mention is not pinned and refuses: a group (it does not compile), an actionref that states a guard of its own, a control
/// with a guard of its own over a guarded field. Read from the object's text, so a mention in a comment refuses too: only a
/// restricted session is affected.
/// The controls that took the guard of their table field carry the marker property [`FIELD_GUARD`].
pub(crate) fn access_by_permission_refusals(
    ws: &crate::workspace::Workspace,
    oi: usize,
    obj: &ast::Object,
    controls: &[ir::Control],
) -> Vec<String> {
    // The object's text is a part of its file's text (the preprocessor only removes), so a file without the word has no
    // mention: the whole-object text, lowercased, is made only for the files that name it.
    if !ws.names_access_by_permission(ws.objects[oi].file) {
        return Vec::new();
    }
    let marked = |c: &ir::Control, m: &str| c.props.iter().any(|(n, _)| n == m);
    let inherited = controls.iter().filter(|c| marked(c, FIELD_GUARD)).count();
    let mentions = obj
        .syntax()
        .text()
        .to_string()
        .to_ascii_lowercase()
        .matches("accessbypermission")
        .count();
    if mentions == 0 && inherited == 0 {
        return Vec::new();
    }
    let guarded = |c: &ir::Control| c.prop("accessbypermission").is_some();
    let object_level = obj
        .properties()
        .filter(|p| p.name().eq_ignore_ascii_case("accessbypermission"))
        .count();
    if let Some(c) = controls.iter().find(|c| marked(c, FIELD_GUARD_CONFLICT)) {
        return vec![format!(
            "control `{}` of page `{}` states AccessByPermission over a table field that states one too (which one BC applies is not pinned, ADR-062 probe P21)",
            c.name,
            obj.name()
        )];
    }
    // An actionref with a guard of its own: no corpus test pins whose guard BC applies.
    if let Some(c) = controls.iter().find(|c| c.action_ref.is_some() && guarded(c)) {
        return vec![format!(
            "actionref `{}` of page `{}` states AccessByPermission of its own, which BC applies to the permissions of a restricted session (not pinned, ADR-062 probe P21)",
            c.name,
            obj.name()
        )];
    }
    let pinned = object_level
        + controls
            .iter()
            .filter(|c| {
                c.owner.is_none()
                    && c.action_ref.is_none()
                    && guarded(c)
                    && matches!(
                        c.kind,
                        ir::ControlKind::Field | ir::ControlKind::Action | ir::ControlKind::Part
                    )
            })
            .count()
        - inherited;
    if pinned == mentions {
        Vec::new()
    } else {
        vec![format!(
            "page `{}` has AccessByPermission on something other than a field, action or part of its own layout or the page itself (a group, a comment), which BC applies to the permissions of a restricted session (not pinned, ADR-062 probe P21)",
            obj.name()
        )]
    }
}

/// Marker property of a page control that took the AccessByPermission guard of its table field (`with_field_guard`).
const FIELD_GUARD: &str = "__abp_from_field";
/// Marker property of a page control that states a guard over a table field that states one too.
const FIELD_GUARD_CONFLICT: &str = "__abp_conflict";

/// The properties of a page control bound to a field of the source table: a field with an AccessByPermission property gives its guard
/// to the control (Y84PermAbp.al Y84_Abp_LettersA `TF`: every control over the field is left off the page when the sets do not
/// grant it). A control that states a guard of its own over such a field is marked for refusal: which one BC applies is not pinned.
pub(crate) fn with_field_guard(
    members: &crate::members::MemberTable<'_>,
    table: Option<usize>,
    binding: &ir::Binding,
    owner: Option<usize>,
    mut props: Vec<(String, ir::PropVal)>,
) -> Vec<(String, ir::PropVal)> {
    let (Some(table), ir::Binding::Field(pos), None) = (table, binding, owner) else {
        return props;
    };
    let Some((_, guard)) = members[table].fields[*pos]
        .properties
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("AccessByPermission"))
    else {
        return props;
    };
    let own = props.iter().find(|(n, _)| n == "accessbypermission");
    // The same guard on the control and on its field says what either says alone (Microsoft's pages state both).
    if own.is_some_and(|(_, v)| matches!(v, ir::PropVal::Lit(t) if t.trim().eq_ignore_ascii_case(guard.trim()))) {
        // nothing to add
    } else if own.is_some() {
        props.push((FIELD_GUARD_CONFLICT.into(), ir::PropVal::Lit(String::new())));
    } else {
        props.push(("accessbypermission".into(), ir::PropVal::Lit(guard.clone())));
        props.push((FIELD_GUARD.into(), ir::PropVal::Lit(String::new())));
    }
    props
}

/// An enum's ordinal and text pairs as the IR keeps them.
pub(crate) fn enum_pairs(list: &[(u32, String)]) -> Vec<(i32, String)> {
    list.iter().map(|(v, n)| (*v as i32, n.clone())).collect()
}

/// The own enum values, captions and `CaptionML` lists of an object's members.
pub(crate) fn enum_own(m: &crate::members::Members) -> super::localize::EnumOwn {
    (
        enum_pairs(&m.enum_values),
        enum_pairs(&m.enum_captions),
        enum_pairs(&m.enum_captions_ml),
    )
}
