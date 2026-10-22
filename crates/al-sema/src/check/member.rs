//! Members of a type: page controls, record fields, sections.
use super::*;

impl<'a> Checker<'a> {
    /// The control `name` of page (or request page) `oi`, typed as a test client handle.
    pub(crate) fn control_of_section(&self, oi: usize, name: &str) -> Option<E> {
        let sec = self.named_section(oi, name)?;
        Some(E::Val(match sec.name().to_ascii_lowercase().as_str() {
            "dataitem" => match crate::types::section_object_name(&sec, 1) {
                Some(table) => Ty::TestFilter(table.into()),
                None => Ty::Unknown,
            },
            "part" => match crate::types::section_object_name(&sec, 1) {
                Some(page) => Ty::TestPart(page.into()),
                None => Ty::Unknown,
            },
            "systempart" => Ty::Builtin("TestPart".into()),
            "action" | "actionref" | "customaction" | "systemaction" | "fileuploadaction" => {
                Ty::Builtin("TestAction".into())
            }
            _ => Ty::Builtin("TestField".into()),
        }))
    }

    /// `TP.Name` as a control of the page `TP` shows, if it has one.
    pub(crate) fn page_control_member(&mut self, sc: &Scope, t: &Ty, name: &str) -> Option<E> {
        let (Ty::TestPage(p) | Ty::TestRequestPage(p) | Ty::TestPart(p)) = t else {
            return None;
        };
        let kind = if matches!(t, Ty::TestRequestPage(_)) {
            ObjKind::Report
        } else {
            ObjKind::Page
        };
        let oi = self.resolve(sc, kind, p)?;
        self.control_of_section(oi, name)
    }

    /// Members of a value of type `t`: fields, procedures, library methods.
    pub(crate) fn member(&mut self, sc: &Scope, t: &Ty, name: &str) -> Option<E> {
        match t {
            // Members of a .NET type come from reflection at run time: any name, any arity,
            // a result whose type is known only then (ADR-014).
            Ty::DotNet(alias) => {
                // A member the runtime provides itself declares its result type (ADR-014).
                let ret = self
                    .ws
                    .resolve_dotnet(sc.file, alias)
                    .and_then(|i| al_builtins::dotnet_sigs::lookup(&self.ws.dotnet[i].decl.type_name, name))
                    .map_or_else(
                        || Ty::DotNet("".into()),
                        |r| match r {
                            al_builtins::dotnet_sigs::Ret::Boolean => Ty::Boolean,
                            al_builtins::dotnet_sigs::Ret::Integer => Ty::Integer,
                            al_builtins::dotnet_sigs::Ret::Text => Ty::Text(None),
                            al_builtins::dotnet_sigs::Ret::Object | al_builtins::dotnet_sigs::Ret::Void => {
                                Ty::DotNet("".into())
                            }
                        },
                    );
                return Some(E::Callable(vec![Cand {
                    min: 0,
                    max: usize::MAX,
                    ret,
                    target: Res::DotNetMember(name.to_string()),
                    param_tys: Vec::new(),
                    fields: Vec::new(),
                    rest_field: false,
                    hidden: None,
                    with_off: None,
                }]));
            }
            Ty::Unknown | Ty::Variant if receiver_of(t).is_none() => return Some(E::Val(Ty::Unknown)),
            // A RecordRef writes to a table that is known only when the code runs; the write raises
            // the platform's database events whatever the table is.
            Ty::Builtin(b) if b.eq_ignore_ascii_case("RecordRef") || b.eq_ignore_ascii_case("FieldRef") => {
                self.touch_recref_ops(demand::record_ops(name));
            }
            Ty::Record { table, .. } => {
                let Some(ti) = self.resolve(sc, ObjKind::Table, table) else {
                    return Some(E::Val(Ty::Unknown));
                };
                // Writing runs the table's triggers and the subscribers to its events.
                self.touch_ops(ti, demand::record_ops(name));
                if let Some(f) = self.members[ti].field(name) {
                    return Some(E::Val(f.ty.clone()));
                }
                let procs: Vec<Cand> = self.members[ti]
                    .procedures_named(name)
                    .iter()
                    .map(|p| proc_cand(p))
                    .collect();
                if !procs.is_empty() {
                    // A table procedure Init(A; B) sits next to the built-in Init(): `Row.Init()`, a bare `Init()` and
                    // `Rec.Init()` in the table call the built-in, `Init(3, 'x')` the procedure; a procedure with the
                    // built-in's own signature hides it (record/lt-record-write/RcBInit.al:445, :475, fixtures
                    // QzInitOverload.al). Only Init is pinned: other built-ins stay hidden by a procedure of their name.
                    let mut procs = procs;
                    if name.eq_ignore_ascii_case("Init") {
                        if let Some(recv) = receiver_of(t) {
                            let platform = self.catalog_cands(&recv, name, true, t);
                            let own: Vec<(usize, usize)> = procs.iter().map(|p| (p.min, p.max)).collect();
                            procs.extend(
                                platform
                                    .into_iter()
                                    .filter(|b| !own.iter().any(|(lo, hi)| *lo <= b.max && b.min <= *hi)),
                            );
                        }
                    }
                    return Some(E::Callable(procs));
                }
            }
            Ty::Object(kind, oname) => {
                let Some(oi) = self.resolve(sc, *kind, oname) else {
                    return Some(E::Val(Ty::Unknown));
                };
                if *kind == ObjKind::Codeunit && name.eq_ignore_ascii_case("Run") {
                    self.reach_run(oi);
                }
                let mut procs: Vec<Cand> = self.members[oi]
                    .procedures_named(name)
                    .iter()
                    .map(|p| proc_cand(p))
                    .collect();
                // An interface also has the procedures of the interfaces it extends
                // (codeunit/TestInterfaceRuntimeContracts.al:Interface_ExtendedInterface_IsAndAs).
                if procs.is_empty() && *kind == ObjKind::Interface {
                    for b in self.ws.interface_bases(oi) {
                        procs = self.members[b]
                            .procedures_named(name)
                            .iter()
                            .map(|p| proc_cand(p))
                            .collect();
                        if !procs.is_empty() {
                            break;
                        }
                    }
                }
                if !procs.is_empty() {
                    // A page's own procedure of the name of a platform method (Service Lines declares `SetSelectionFilter()`
                    // and calls `CurrPage.SetSelectionFilter(ServiceLine)`): the platform overloads whose arity the
                    // page's procedures do not take are candidates too.
                    if *kind == ObjKind::Page {
                        if let Some(recv) = receiver_of(t) {
                            let platform = self.catalog_cands(&recv, name, true, t);
                            let own: Vec<(usize, usize)> = procs.iter().map(|p| (p.min, p.max)).collect();
                            procs.extend(
                                platform
                                    .into_iter()
                                    .filter(|b| !own.iter().any(|(lo, hi)| *lo <= b.max && b.min <= *hi)),
                            );
                        }
                    }
                    return Some(E::Callable(procs));
                }
                // `Rep.Run()`, `Xp.Import()`: the platform's methods of a report or XML port run its
                // triggers, so its code is reached.
                if matches!(kind, ObjKind::Report | ObjKind::XmlPort) {
                    self.touch_full(oi);
                }
                // `CurrPage.<Part>`: the part control, whose `.Page` is the page it shows.
                if *kind == ObjKind::Page {
                    // `P.Run()`, `P.RunModal()`: a platform method of a page variable opens the
                    // page, so its code is reached (a declared variable alone reaches nothing).
                    if self.named_section(oi, name).is_none() {
                        self.touch_full(oi);
                        self.reach_run_targets(oi);
                    }
                    if let Some(sec) = self.named_section(oi, name) {
                        if sec.name().eq_ignore_ascii_case("part") {
                            if let Some(page) = crate::types::section_object_name(&sec, 1) {
                                return Some(E::Val(Ty::TestPart(page.into())));
                            }
                        }
                    }
                }
                if *kind == ObjKind::Query {
                    if let Some(l) = self.query(oi) {
                        if let Some(c) = l.column(name) {
                            return Some(E::Val(l.column_tys[c].clone()));
                        }
                    }
                }
                if matches!(
                    kind,
                    ObjKind::Page | ObjKind::Query | ObjKind::Report | ObjKind::XmlPort
                ) && self.has_named_section(oi, name)
                {
                    return Some(E::Val(Ty::Unknown));
                }
            }
            Ty::TestPage(p) | Ty::TestRequestPage(p) | Ty::TestPart(p) => {
                // Built-in test page members (`OK`, `Close`, `Filter`, ...) win over
                // controls of the same name: corpus `Modal.OK().Invoke()` compiles.
                let kind = if matches!(t, Ty::TestRequestPage(_)) {
                    ObjKind::Report
                } else {
                    ObjKind::Page
                };
                let resolved = self.resolve(sc, kind, p);
                let recv = receiver_of(t).expect("test page types have a receiver");
                let mut c = self.catalog_cands(&recv, name, true, t);
                if !c.is_empty() {
                    // `TP.Filter` filters the page's source table.
                    let filter = resolved
                        .and_then(|o| self.source_table(o))
                        .map(|t| Ty::TestFilter(t.into()))
                        .unwrap_or(Ty::Unknown);
                    for cand in &mut c {
                        if cand.ret == Ty::Builtin("TestFilter".into()) {
                            cand.ret = filter.clone();
                        }
                    }
                    return Some(E::Callable(c));
                }
                if let (Ty::TestPart(part), true) = (t, name.eq_ignore_ascii_case("Page")) {
                    if resolved.is_some() {
                        return Some(E::Val(Ty::Object(ObjKind::Page, part.clone())));
                    }
                }
                if let Some(oi) = resolved {
                    if let Some(e) = self.control_of_section(oi, name) {
                        return Some(e);
                    }
                } else {
                    return Some(E::Val(Ty::Unknown));
                }
            }
            _ => {}
        }
        let recv = receiver_of(t)?;
        let c = self.catalog_cands(&recv, name, true, t);
        if !c.is_empty() {
            return Some(E::Callable(c));
        }
        // Label and TextConst values have the Text methods too.
        if matches!(t, Ty::Label) {
            let c = self.catalog_cands("Text", name, true, t);
            if !c.is_empty() {
                return Some(E::Callable(c));
            }
        }
        None
    }

    /// The type of field `name` of the table behind `t` (a record, a test
    /// page's filter, or a query's column).
    pub(crate) fn field_of(&self, sc: &Scope, t: &Ty, name: &str) -> Option<Ty> {
        match t {
            Ty::Record { table, .. } => match self.resolve(sc, ObjKind::Table, table) {
                Some(ti) => self.members[ti].field(name).map(|f| f.ty.clone()),
                // The table is outside the workspace (already reported); do not guess.
                None => Some(Ty::Unknown),
            },
            Ty::Unknown => Some(Ty::Unknown),
            Ty::TestFilter(table) => self.field_of(
                sc,
                &Ty::Record {
                    table: table.clone(),
                    temporary: false,
                },
                name,
            ),
            Ty::TestPage(page) | Ty::TestPart(page) => {
                let Some(pi) = self.resolve(sc, ObjKind::Page, page) else {
                    return Some(Ty::Unknown);
                };
                let table = self.source_table(pi)?;
                self.field_of(
                    sc,
                    &Ty::Record {
                        table: table.into(),
                        temporary: false,
                    },
                    name,
                )
            }
            Ty::Object(ObjKind::Query, q) => {
                let qi = self.resolve(sc, ObjKind::Query, q)?;
                if let Some(c) = self.query(qi).and_then(|l| l.column(name).map(|c| (l, c))) {
                    return Some(c.0.column_tys[c.1].clone());
                }
                self.has_named_section(qi, name).then_some(Ty::Unknown)
            }
            _ => None,
        }
    }

    /// A named control/data item/column of object `oi` or of one of its extensions.
    pub(crate) fn named_section(&self, oi: usize, name: &str) -> Option<ast::Section> {
        std::iter::once(oi)
            .chain(self.ws.extensions_of(oi).iter().copied())
            .find_map(|o| {
                self.sections
                    .borrow_mut()
                    .entry(o)
                    .or_insert_with(|| {
                        ast::sections_in(self.ws.object_ast(o).syntax())
                            .into_iter()
                            .map(|s| (s.args().into_iter().next(), s))
                            .collect()
                    })
                    .iter()
                    .find(|(first, _)| first.as_ref().is_some_and(|a| a.eq_ignore_ascii_case(name)))
                    .map(|(_, s)| s.clone())
            })
    }

    pub(crate) fn source_table(&self, page: usize) -> Option<String> {
        let v = self
            .ws
            .object_ast(page)
            .properties()
            .find(|p| p.name().eq_ignore_ascii_case("SourceTable"))?
            .value_text();
        Some(v.trim().trim_matches('"').to_string())
    }

    pub(crate) fn has_named_section(&self, oi: usize, name: &str) -> bool {
        self.named_section(oi, name).is_some()
    }
}
