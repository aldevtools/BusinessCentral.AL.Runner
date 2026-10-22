//! The checker's object context, name resolution, and the walk over an object's routines.
use super::*;

impl<'a> Checker<'a> {
    pub(crate) fn new(
        ws: &'a Workspace,
        members: &'a MemberTable<'a>,
        catalog: &'a Catalog,
        system_enums: HashSet<String>,
        demand_mode: bool,
    ) -> Checker<'a> {
        Checker {
            ws,
            members,
            catalog,
            diags: Vec::new(),
            types: Default::default(),
            resolutions: Default::default(),
            system_enums,
            sections: Default::default(),
            queries: (0..ws.objects.len()).map(|_| std::cell::OnceCell::new()).collect(),
            calls: Vec::new(),
            demand: std::cell::RefCell::new(demand_mode.then(|| demand::State::new(ws.objects.len()))),
            prefetch: None,
            resolved: Default::default(),
            panicked: HashMap::default(),
            cur_ret: None,
            tdd_apps: None,
            tdd_pending: Vec::new(),
            all_routines: false,
        }
    }

    /// Runs `f`; if it panics, the panic is recorded against `key` (object, routine offset) and the
    /// check goes on with the next piece of work. The checker's state after a panic only ever holds
    /// more than the routine reached (types, calls, reached objects), never less.
    pub(crate) fn guard(&mut self, key: (usize, usize), f: impl FnOnce(&mut Self)) {
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(self))) {
            self.panicked.insert(key, panic_message(&*p));
        }
    }

    pub(crate) fn diag(&mut self, sc: &Scope, node: &SyntaxNode, code: &'static str, message: String) {
        self.diags.push(Diagnostic {
            file: self.ws.files[sc.file].path.clone(),
            app: self.ws.files[sc.file].app,
            offset: node.text_range().start().into(),
            code,
            message,
        });
    }

    /// [`record`](Self::record) for a name a `with` statement supplied.
    pub(crate) fn record_with(&mut self, sc: &Scope, node: &SyntaxNode, r: Res, with: Option<usize>) {
        let r = match with {
            Some(with) => Res::With {
                with,
                inner: Box::new(r),
            },
            None => r,
        };
        self.record(sc, node, r);
    }

    pub(crate) fn record(&mut self, sc: &Scope, node: &SyntaxNode, r: Res) {
        if let Res::Proc { object, offset } = r.inner() {
            self.calls.push((*object, *offset));
        }
        let range = node.text_range();
        self.resolutions
            .insert((sc.file, range.start().into(), range.end().into()), r);
    }

    /// `Res::Field` if `name` is a field of the record type `t`.
    pub(crate) fn field_res(&self, sc: &Scope, t: &Ty, name: &str) -> Option<Res> {
        if let Ty::Object(ObjKind::Query, q) = t {
            let qi = self.resolve(sc, ObjKind::Query, q)?;
            let column = self.query(qi)?.column(name)?;
            return Some(Res::QueryColumn { query: qi, column });
        }
        // A `TestPage.Filter` names the fields of the page's source table.
        let (Ty::Record { table, .. } | Ty::TestFilter(table)) = t else {
            return None;
        };
        let ti = self.resolve(sc, ObjKind::Table, table)?;
        self.members[ti].field(name).map(|f| Res::Field { table: ti, id: f.id })
    }

    pub(crate) fn resolve(&self, sc: &Scope, kind: ObjKind, name: &str) -> Option<usize> {
        self.resolve_at(sc.file, kind, name)
    }

    /// Resolves an object reference written in `file`. Whatever it finds is used by the code
    /// being checked, so a dependency object found this way is reached (see `demand`).
    pub(crate) fn resolve_at(&self, file: usize, kind: ObjKind, name: &str) -> Option<usize> {
        let found = self.resolve_quiet(file, kind, name)?;
        self.touch(found);
        Some(found)
    }

    /// `resolve_at` without marking the object as used.
    pub(crate) fn resolve_quiet(&self, file: usize, kind: ObjKind, name: &str) -> Option<usize> {
        if let Some(hit) = self.resolved.borrow().get(&(file, kind)).and_then(|m| m.get(name)) {
            return *hit;
        }
        let parts: Vec<String> = if name.parse::<u32>().is_ok() {
            vec![name.to_string()]
        } else {
            split_qualified(name)
        };
        let found = match self.ws.resolve_ref(file, kind, &parts) {
            Resolution::Found(i) => Some(i),
            _ => None,
        };
        self.resolved
            .borrow_mut()
            .entry((file, kind))
            .or_default()
            .insert(name.to_string(), found);
        found
    }

    /// Like `resolve_at`, for a reference already split into its parts.
    pub(crate) fn resolve_parts(&self, file: usize, kind: ObjKind, parts: &[String]) -> Option<usize> {
        match self.ws.resolve_ref(file, kind, parts) {
            Resolution::Found(i) => {
                self.touch(i);
                Some(i)
            }
            _ => None,
        }
    }

    /// The layout of query object `qi`.
    pub(crate) fn query(&self, qi: usize) -> Option<&crate::queries::QueryLayout> {
        self.queries[qi]
            .get_or_init(|| {
                (self.ws.objects[qi].kind == ObjKind::Query && self.ws.usable(qi))
                    .then(|| crate::queries::layout(self.ws, self.members, qi))
            })
            .as_ref()
    }

    /// What every routine of object `oi` sees: the object's own variables, the implicit
    /// record, and the data items of a report extension's base.
    pub(crate) fn object_ctx(&mut self, oi: usize) -> ObjCtx {
        let obj = self.ws.object_ast(oi);
        let o = &self.ws.objects[oi];
        let mut globals = super::posmap::FxMap::default();
        for v in obj.var_decls() {
            let ty = v.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown);
            for n in v.names() {
                globals.insert(n.to_lowercase(), ty.clone());
            }
        }
        // The variables of a report's `requestpage` section are the globals of its request page; only its triggers
        // see them (corpus report/lt-ab-request-page-triggers/ABRequestPageExt.al:63, a request page extension that
        // declares the same names as its base report). A report-level variable of the same name stays.
        for v in obj.request_page_var_decls() {
            let ty = v.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown);
            for n in v.names() {
                globals.entry(n.to_lowercase()).or_insert_with(|| ty.clone());
            }
        }
        // Report/query/xmlport data items are record variables in the object's scope.
        let sections = ast::sections_in(obj.syntax());
        for sec in &sections {
            let n = sec.name().to_ascii_lowercase();
            if n == "dataitem" || n == "tableelement" {
                let args = sec.args();
                if let (Some(name), Some(table)) = (args.first(), crate::types::section_object_name(sec, 1)) {
                    globals.insert(
                        name.to_lowercase(),
                        Ty::Record {
                            table: table.into(),
                            temporary: false,
                        },
                    );
                }
            }
        }
        for sec in &sections {
            let n = sec.name().to_ascii_lowercase();
            if n == "textelement" || n == "textattribute" {
                if let Some(name) = sec.args().first() {
                    // TextType = BigText makes the element a BigText variable (corpus xmlport/lt-xmlport-bigtext/XmlPortBigText.al).
                    let big = sec.properties().any(|p| {
                        p.name().eq_ignore_ascii_case("TextType")
                            && p.value_text().trim().eq_ignore_ascii_case("BigText")
                    });
                    let ty = if big {
                        Ty::Builtin("BigText".into())
                    } else {
                        Ty::Text(None)
                    };
                    globals.insert(name.to_lowercase(), ty);
                }
            }
        }
        if matches!(
            o.kind,
            ObjKind::Table | ObjKind::TableExtension | ObjKind::Page | ObjKind::PageExtension
        ) {
            // System variable: the field number whose validation is running.
            globals.insert("currfieldno".into(), Ty::Integer);
        }
        let prop = |name: &str| {
            obj.properties()
                .find(|p| p.name().eq_ignore_ascii_case(name))
                .map(|p| p.value_text())
        };
        let unquote = |s: String| s.trim().trim_matches('"').to_string();
        let mut rec = None;
        let mut rec_unqualified = false;
        let mut self_obj = oi;
        match o.kind {
            ObjKind::Table => {
                rec = Some(Ty::Record {
                    table: o.name.as_str().into(),
                    temporary: false,
                });
                rec_unqualified = true;
            }
            ObjKind::TableExtension => {
                if let Some(base) = obj.extends_parts() {
                    if let Some(t) = self.resolve_parts(o.file, ObjKind::Table, &base) {
                        rec = Some(Ty::Record {
                            table: self.ws.objects[t].name.as_str().into(),
                            temporary: false,
                        });
                        self_obj = t;
                    } else {
                        rec = Some(Ty::Unknown);
                    }
                }
                rec_unqualified = true;
            }
            ObjKind::Page => {
                rec = prop("SourceTable").map(|t| Ty::Record {
                    table: unquote(t).into(),
                    temporary: false,
                });
                rec_unqualified = true;
                globals.insert("currpage".into(), Ty::Object(ObjKind::Page, o.name.as_str().into()));
            }
            ObjKind::PageExtension => {
                if let Some(base) = obj.extends_parts() {
                    if let Some(p) = self.resolve_parts(o.file, ObjKind::Page, &base) {
                        let pobj = self.ws.object_ast(p);
                        rec = pobj
                            .properties()
                            .find(|x| x.name().eq_ignore_ascii_case("SourceTable"))
                            .map(|x| Ty::Record {
                                table: unquote(x.value_text()).into(),
                                temporary: false,
                            });
                        globals.insert(
                            "currpage".into(),
                            Ty::Object(ObjKind::Page, self.ws.objects[p].name.as_str().into()),
                        );
                    } else {
                        rec = Some(Ty::Unknown);
                        globals.insert("currpage".into(), Ty::Unknown);
                    }
                }
                rec_unqualified = true;
            }
            ObjKind::Codeunit => {
                rec = prop("TableNo").map(|t| Ty::Record {
                    table: unquote(t).into(),
                    temporary: false,
                });
                // The fields and methods of the implicit Rec can be used unqualified in a codeunit with TableNo
                // (Microsoft's "Save Data Exch. Blob Sample": `"File Name" := ...; Find(); CalcFields(..)`).
                rec_unqualified = rec.is_some();
            }
            ObjKind::Report | ObjKind::ReportExtension => {
                // In a report extension `CurrReport` is the report run of the extended report, so its
                // members resolve against the base report, not against a report named like the extension
                // (Tests-ERM "Mfg. Document Entries", issue 19).
                let report = match (o.kind, obj.extends_parts()) {
                    (ObjKind::ReportExtension, Some(base)) => self
                        .resolve_parts(o.file, ObjKind::Report, &base)
                        .map_or_else(|| o.name.clone(), |r| self.ws.objects[r].name.clone()),
                    _ => o.name.clone(),
                };
                globals.insert("currreport".into(), Ty::Object(ObjKind::Report, report.into()));
                globals.insert("requestoptionspage".into(), Ty::Builtin("RequestPage".into()));
                // The request page of a report states a `SourceTable`: its triggers and controls name
                // that table's current record as `Rec` (handlers/TestReportRequestPageSourceTable
                // CompiledContracts.al). The checker only needs the name to resolve; running such a
                // report with a request page Rec is refused by the lowerer.
                if o.kind == ObjKind::Report {
                    let source = ast::sections_in(obj.syntax())
                        .into_iter()
                        .filter(|sec| sec.name().eq_ignore_ascii_case("requestpage"))
                        .find_map(|sec| {
                            sec.properties()
                                .find(|p| p.name().eq_ignore_ascii_case("SourceTable"))
                                .map(|p| unquote(p.value_text()))
                        });
                    if let Some(table) = source {
                        rec = Some(Ty::Record {
                            table: table.into(),
                            temporary: false,
                        });
                    }
                }
            }
            ObjKind::XmlPort => {
                globals.insert(
                    "currxmlport".into(),
                    Ty::Object(ObjKind::XmlPort, o.name.as_str().into()),
                );
            }
            // A query's procedures call the query's own methods unqualified and name its columns
            // and filters (query/TestQueryDataItemTableFilterDatabaseConst.al: `SetSourceFilter` of
            // Base App's query 522 does `SetRange(Source_Type, ..)`).
            ObjKind::Query => {
                rec = Some(Ty::Object(ObjKind::Query, o.name.as_str().into()));
                rec_unqualified = true;
                // `CurrQuery` is the running query in its triggers (query/TestQueryOnBeforeOpenVariablesContracts.al:75-77).
                globals.insert("currquery".into(), Ty::Object(ObjKind::Query, o.name.as_str().into()));
            }
            _ => {}
        }
        if let Some(r) = &rec {
            globals.insert("rec".into(), r.clone());
            globals.insert("xrec".into(), r.clone());
        }
        // Report extensions see the base report's data items; `modify(X)` blocks
        // run with data item X as their record.
        let mut base_items: HashMap<String, String> = HashMap::default();
        if o.kind == ObjKind::ReportExtension {
            if let Some(base) = obj.extends_parts() {
                if let Some(r) = self.resolve_parts(o.file, ObjKind::Report, &base) {
                    for sec in ast::sections_in(self.ws.object_ast(r).syntax()) {
                        if sec.name().eq_ignore_ascii_case("dataitem") {
                            let a = sec.args();
                            if let (Some(n), Some(t)) = (a.first(), crate::types::section_object_name(&sec, 1)) {
                                base_items.insert(n.to_lowercase(), t.clone());
                                globals.insert(
                                    n.to_lowercase(),
                                    Ty::Record {
                                        table: t.into(),
                                        temporary: false,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
        // ADR-023: a page or report extension sees the base object's `protected var` variables
        // (those the extension does not declare itself) and its procedures.
        let mut base_obj = None;
        let mut base_vars = std::collections::HashSet::default();
        if crate::base_access::scope_enabled() {
            let base_kind = match o.kind {
                ObjKind::PageExtension => Some(ObjKind::Page),
                ObjKind::ReportExtension => Some(ObjKind::Report),
                _ => None,
            };
            if let (Some(kind), Some(parts)) = (base_kind, obj.extends_parts()) {
                if let Some(b) = self.resolve_parts(o.file, kind, &parts) {
                    base_obj = Some(b);
                    for (name, decl) in crate::base_access::visible_vars(&self.ws.object_ast(b)) {
                        if globals.contains_key(&name) {
                            continue;
                        }
                        let ty = decl.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown);
                        globals.insert(name.clone(), ty);
                        base_vars.insert(name);
                    }
                }
            }
        }
        // A table extension sees the protected variables of its base table (assumption `table-extension-base-vars`:
        // tableextension 906 "Asm. Requisition Line" of Microsoft's Base Application). Only the variables: the base
        // table's procedures keep resolving through the implicit Rec, so `base_obj` stays unset.
        if o.kind == ObjKind::TableExtension && crate::base_access::scope_enabled() {
            if let Some(parts) = obj.extends_parts() {
                if let Some(b) = self.resolve_parts(o.file, ObjKind::Table, &parts) {
                    for (name, decl) in crate::base_access::visible_vars(&self.ws.object_ast(b)) {
                        if globals.contains_key(&name) {
                            continue;
                        }
                        let ty = decl.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown);
                        globals.insert(name.clone(), ty);
                        base_vars.insert(name);
                    }
                }
            }
        }
        let file = o.file;
        // Every type the object's variables name is used by the code that runs in it.
        let tys: Vec<Ty> = globals.values().cloned().chain(rec.clone()).collect();
        for t in &tys {
            self.touch_ty(file, t);
        }
        ObjCtx {
            oi,
            file,
            globals: std::rc::Rc::new(globals),
            rec,
            rec_unqualified,
            self_obj,
            base_items,
            base_obj,
            base_vars: std::rc::Rc::new(base_vars),
        }
    }

    /// AL0132 (corpus must-not-compile 0212): a page field whose source is `Rec.Name`, where `Name` is no
    /// field or procedure of the source table and no method of `Record`. Only a table of the workspace that no
    /// extension adds to is judged, and only a plain `Rec.Name` source (the expression of a page control is
    /// not checked as code, see `pages::is_synthetic`).
    fn al_page_field_sources(&mut self, oi: usize, obj: &ast::Object) {
        if self.ws.objects[oi].kind != ObjKind::Page || self.ws.is_dependency(oi) {
            return;
        }
        let file = self.ws.objects[oi].file;
        let Some(table) = self.source_table(oi) else { return };
        let Some(ti) = self.resolve_quiet(file, ObjKind::Table, &table) else {
            return;
        };
        if self.ws.is_dependency(ti) || !self.ws.extensions_of(ti).is_empty() {
            return;
        }
        for sec in crate::pages::control_sections(obj) {
            if !sec.name().eq_ignore_ascii_case("field") {
                continue;
            }
            let Some(src) = sec.args_raw().get(1).cloned() else {
                continue;
            };
            let Some(rest) = src
                .get(..4)
                .filter(|p| p.eq_ignore_ascii_case("Rec."))
                .map(|_| src[4..].trim())
            else {
                continue;
            };
            let name = match rest.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
                Some(q) if !q.contains('"') => q,
                Some(_) => continue,
                None if !rest.is_empty() && rest.chars().all(|c| c.is_alphanumeric() || c == '_') => rest,
                None => continue,
            };
            if self.members[ti].field(name).is_some()
                || !self.members[ti].procedures_named(name).is_empty()
                || !self.catalog.overloads("Record", name).is_empty()
                || !self.catalog.hidden("Record", name).is_empty()
            {
                continue;
            }
            let table_name = self.ws.objects[ti].name.clone();
            self.diags.push(Diagnostic {
                file: self.ws.files[file].path.clone(),
                app: self.ws.files[file].app,
                offset: usize::from(sec.syntax().text_range().start()),
                code: "AL0132",
                message: format!("'Record \"{table_name}\"' does not contain a definition for '{name}'"),
            });
        }
    }

    pub(crate) fn check_object(&mut self, oi: usize) {
        let ctx = self.object_ctx(oi);
        let obj = self.ws.object_ast(oi);
        self.al_page_field_sources(oi, &obj);
        for r in ast::routines_in(obj.syntax()) {
            let offset = usize::from(r.syntax().text_range().start());
            if !self.all_routines && self.ws.routine_unreached(oi, offset) {
                continue;
            }
            self.guard((oi, offset), |c| c.check_routine_in(&ctx, &r));
        }
        self.reach_filter_helper_if_used(oi);
        self.reach_save_as_events_if_used(oi);
        // What the test client does when it opens a workspace page or report, and what its actions
        // run, reaches into the dependency apps.
        if matches!(
            self.ws.objects[oi].kind,
            ObjKind::Page | ObjKind::PageExtension | ObjKind::Report | ObjKind::ReportExtension
        ) {
            self.reach_caption_class_if_used(oi);
            self.reach_reporting_triggers_if_used(oi);
        }
        if matches!(self.ws.objects[oi].kind, ObjKind::Page | ObjKind::PageExtension) {
            self.reach_run_objects(oi);
            // The client writes to the page's table (validating a field, leaving a new row): the
            // table's triggers and events run, whichever app the table is in.
            if self.demand.borrow().is_some() {
                self.activate_source_table(oi);
            }
        }
    }

    /// The locals of routine `r` (by lower-case name) that are records of a platform table and are
    /// named in the body only as `Name.Field` with `Field` a field of that table: nothing is asked
    /// of the table, so the variable does not stand for its rows.
    pub(crate) fn field_only_records(
        &self,
        file: usize,
        r: &ast::Routine,
        locals: &super::posmap::FxMap<String, Ty>,
    ) -> HashSet<String> {
        let mut out = HashSet::default();
        let Some(body) = r.body() else { return out };
        for (name, t) in locals {
            let Ty::Record { table, .. } = t else { continue };
            let Some(ti) = self.resolve_quiet(file, ObjKind::Table, table) else {
                continue;
            };
            if !self.ws.platform_table(ti) {
                continue;
            }
            let only_fields = body
                .descendants()
                .filter(|n| n.kind() == SyntaxKind::NameRef && name_of(n).eq_ignore_ascii_case(name))
                .all(|n| {
                    let Some(p) = n.parent().filter(|p| p.kind() == SyntaxKind::MemberExpr) else {
                        return false;
                    };
                    let mut kids = p.children();
                    let (Some(base), Some(member)) = (kids.next(), kids.next()) else {
                        return false;
                    };
                    base == n
                        && self.members[ti].field(&name_of(&member)).is_some()
                        && !p.parent().is_some_and(|g| g.kind() == SyntaxKind::CallExpr)
                });
            if only_fields {
                out.insert(name.clone());
            }
        }
        out
    }

    /// Checks one routine of the object `ctx` describes.
    pub(crate) fn check_routine_in(&mut self, ctx: &ObjCtx, r: &ast::Routine) {
        #[cfg(test)]
        if r.name() == "InjectedCheckPanic" {
            panic!("injected check panic");
        }
        let ObjCtx {
            oi,
            file,
            globals,
            rec,
            rec_unqualified,
            self_obj,
            base_items,
            base_obj,
            base_vars,
        } = ctx;
        let (oi, file, self_obj) = (*oi, *file, *self_obj);
        {
            let item_rec = r.syntax().ancestors().filter_map(ast::Section::cast).find_map(|s| {
                let a = s.args();
                match s.name().to_ascii_lowercase().as_str() {
                    // The table as the object's variables name it, also when it is namespace-qualified.
                    "dataitem" => crate::types::section_object_name(&s, 1),
                    "modify" | "add" => a.first().and_then(|n| base_items.get(&n.to_lowercase()).cloned()),
                    _ => None,
                }
            });
            let report_item = item_rec.is_some()
                && self.ws.objects[oi].kind == ObjKind::Report
                && r.syntax()
                    .ancestors()
                    .filter_map(ast::Section::cast)
                    .any(|s| s.name().eq_ignore_ascii_case("dataitem"));
            let (rec, rec_unqualified) = match item_rec {
                Some(t) => (
                    Some(Ty::Record {
                        table: t.into(),
                        temporary: false,
                    }),
                    true,
                ),
                None => (rec.clone(), *rec_unqualified),
            };
            let mut sc = Scope {
                oi,
                file,
                globals: globals.clone(),
                vars: Vec::new(),
                rec,
                rec_unqualified,
                self_obj,
                base_obj: *base_obj,
                base_vars: base_vars.clone(),
                withs: Vec::new(),
                codeunit_on_run: self.ws.objects[oi].kind == ObjKind::Codeunit
                    && r.is_trigger()
                    && r.name().eq_ignore_ascii_case("OnRun"),
                report_item,
            };
            let mut locals = super::posmap::FxMap::default();
            for p in r.params() {
                locals.insert(
                    p.name().to_lowercase(),
                    p.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown),
                );
            }
            if let (Some(n), Some(t)) = (r.return_name(), r.return_type()) {
                locals.insert(n.to_lowercase(), Ty::from_type_ref(&t));
            } else if let Some(t) = r.return_type() {
                // An unnamed return value is used by the code that runs as well (an interface it
                // names must be compiled, or the routine is refused for it).
                self.touch_ty(file, &Ty::from_type_ref(&t));
            }
            for v in r.local_var_decls() {
                let ty = v.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown);
                for n in v.names() {
                    locals.insert(n.to_lowercase(), ty.clone());
                }
            }
            // The types of everything in scope are used by the code that runs here. A record variable
            // of a catalog table (`Field`, `Key`, ...) that only has its fields assigned and read is a
            // buffer in memory: it reads no row, so the shapes of all the objects the table lists are
            // not needed (Library Assert's TypeNameOf is such a routine).
            let buffers = self.field_only_records(file, r, &locals);
            for (name, t) in locals.iter() {
                if buffers.contains(name) {
                    if let Ty::Record { table, .. } = t {
                        self.resolve_at(file, ObjKind::Table, table);
                    }
                } else {
                    self.touch_ty(file, t);
                }
            }
            for t in sc.rec.iter() {
                self.touch_ty(file, t);
            }
            sc.vars.push(locals);
            let before = self.diags.len();
            self.cur_ret = r.return_type().map(|t| Ty::from_type_ref(&t));
            if let Some(body) = r.body() {
                self.walk(&mut sc, &body);
            }
            self.tdd_finish_routine();
            // Property expressions of controls were never checked before the test
            // client needed them (ADR-012); an unresolved name there stays a refusal at
            // lowering, not a compile diagnostic (check::tests::builtin_test_page_methods_win_over_controls_when_called).
            if crate::pages::is_synthetic(&r.name()) {
                self.diags.truncate(before);
            }
        }
    }

    pub(crate) fn walk(&mut self, sc: &mut Scope, node: &SyntaxNode) {
        if is_expr(node.kind()) {
            self.ty(sc, node);
            return;
        }
        if node.kind() == SyntaxKind::WithStmt {
            // `with Rec do Stmt`: the record's members come before every variable and procedure of the code around
            // it (corpus events/lt-with-statement-scopes/WithStatementAmbiguous.al: a field wins over a global, a
            // local, a procedure of the codeunit and a builtin method of the same name; WithStatementNested.al:
            // the inner statement's record first, then the outer's). The record expression itself is read in the
            // scope outside this statement.
            let mut kids = node.children();
            let record = kids.next();
            let pushed = match &record {
                Some(r) if is_expr(r.kind()) => {
                    let t = self.ty(sc, r);
                    // Names are only taken from a record whose table is known: an unresolved one would swallow every
                    // name and hide the unresolved-name diagnostics.
                    let known = match &t {
                        Ty::Record { table, .. } => self.resolve(sc, ObjKind::Table, table).is_some(),
                        Ty::Object(kind, name) => self.resolve(sc, *kind, name).is_some(),
                        _ => false,
                    };
                    known.then(|| {
                        sc.withs.push((t, usize::from(node.text_range().start())));
                    })
                }
                _ => None,
            };
            for child in kids {
                self.walk(sc, &child);
            }
            if pushed.is_some() {
                sc.withs.pop();
            }
            return;
        }
        if node.kind() == SyntaxKind::AssignStmt {
            let mut tys = Vec::new();
            let plain_assign = node.children_with_tokens().any(|t| t.kind() == SyntaxKind::Assign);
            let exprs: Vec<SyntaxNode> = node.children().filter(|c| is_expr(c.kind())).collect();
            for child in node.children() {
                if is_expr(child.kind()) {
                    if plain_assign && exprs.len() == 2 && child == exprs[0] && child.kind() == SyntaxKind::MemberExpr {
                        let t = self.assign_target_ty(sc, &child, &exprs[1]);
                        tys.push((t, child));
                        continue;
                    }
                    tys.push((self.ty(sc, &child), child));
                } else {
                    self.walk(sc, &child);
                }
            }
            if let [(target, lhs), (source, rhs)] = tys.as_slice() {
                let plain = node.children_with_tokens().any(|t| t.kind() == SyntaxKind::Assign);
                if plain {
                    self.al_enum_assignment(sc, rhs, target, source);
                    self.al_integer_text_assignment(sc, rhs, target, source);
                    self.al_scalar_assignment(sc, rhs, target, source);
                    self.al_no_result_assignment(sc, rhs, target, source);
                    self.al_interface_assignment(sc, rhs, target, source);
                }
                // `X := Target.Missing()` and `X += Target.Missing()`: the type of the target (ADR-051 decision 3).
                let (target, rhs) = (target.clone(), rhs.clone());
                self.tdd_settle(sc, &rhs, &target);
                // `Rec.Missing := 5`: a stub field takes the type of the source (decision 10).
                let (source, lhs) = (source.clone(), lhs.clone());
                self.tdd_settle(sc, &lhs, &source);
            }
            return;
        }
        // `Target.Missing();` as a statement: nothing reads a result (ADR-051 decision 3).
        if node.kind() == SyntaxKind::ExprStmt {
            for child in node.children() {
                if is_expr(child.kind()) {
                    self.ty(sc, &child);
                    self.tdd_settle_statement(sc, &child);
                } else {
                    self.walk(sc, &child);
                }
            }
            return;
        }
        // `repeat ... until Target.Missing()`: a Boolean.
        if node.kind() == SyntaxKind::RepeatStmt {
            for child in node.children() {
                if is_expr(child.kind()) {
                    self.ty(sc, &child);
                    self.tdd_settle(sc, &child, &Ty::Boolean);
                } else {
                    self.walk(sc, &child);
                }
            }
            return;
        }
        // `case Target.Missing() of`: the type of the case labels.
        if node.kind() == SyntaxKind::CaseStmt {
            let mut selector: Option<(SyntaxNode, Ty)> = None;
            for child in node.children() {
                if selector.is_none() && is_expr(child.kind()) {
                    let t = self.ty(sc, &child);
                    selector = Some((child, t));
                } else {
                    self.walk(sc, &child);
                }
            }
            if let Some((sel, sel_ty)) = selector {
                // AL0398: a Decimal constant as a label of an Option selector (must-not-compile 2008).
                let labels: Vec<SyntaxNode> = node
                    .children()
                    .filter(|c| c.kind() == SyntaxKind::CaseBranch)
                    .flat_map(|b| b.children().filter(|c| is_expr(c.kind())).collect::<Vec<_>>())
                    .collect();
                self.al_option_decimal_constants(sc, &sel, &sel_ty, &labels);
                self.tdd_settle_case(sc, node, &sel);
            }
            return;
        }
        // AL0122 (corpus must-not-compile 0285, 0286): the condition of `if` and `while` is no Boolean.
        if matches!(node.kind(), SyntaxKind::IfStmt | SyntaxKind::WhileStmt) {
            let mut first = true;
            for child in node.children() {
                if first && is_expr(child.kind()) {
                    first = false;
                    let t = self.ty(sc, &child);
                    self.al_condition(sc, &child, &t);
                    self.tdd_settle(sc, &child, &Ty::Boolean);
                } else {
                    self.walk(sc, &child);
                }
            }
            return;
        }
        // AL0122 (corpus must-not-compile 0274): `exit(value)` of the wrong type.
        if node.kind() == SyntaxKind::ExitStmt {
            for child in node.children() {
                if is_expr(child.kind()) {
                    let t = self.ty(sc, &child);
                    if let Some(ret) = self.cur_ret.clone() {
                        // exit(value) of an enum of another type (corpus must-not-compile 0405).
                        self.al_enum_assignment(sc, &child, &ret, &t);
                        self.al_integer_text_assignment(sc, &child, &ret, &t);
                        self.al_scalar_assignment(sc, &child, &ret, &t);
                        // `exit(Target.Missing())`: the result type of the routine (ADR-051 decision 3).
                        self.tdd_settle(sc, &child, &ret);
                    }
                } else {
                    self.walk(sc, &child);
                }
            }
            return;
        }
        for child in node.children() {
            self.walk(sc, &child);
        }
    }
}
