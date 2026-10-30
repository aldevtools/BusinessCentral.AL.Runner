//! Lowering of page control trees, `RunObject` and part links.
use super::*;

impl Lowerer<'_> {
    /// The control tree of page `oi` with its extensions merged in, with bindings and routine
    /// indices resolved. A control an extension declares keeps the extension as its `owner`: its
    /// routines and variable slots are the extension's.
    pub(crate) fn page(
        &self,
        oi: usize,
        (obj, request_section): (&ast::Object, Option<&ast::Section>),
        layout: &crate::pages::Layout,
        globals: &[ir::Slot],
        reasons: &[String],
    ) -> ir::Page {
        let mut unsupported = reasons.to_vec();
        let mut big_integer_expressions = Vec::new();
        let table = self.table_of[oi];
        let extensions: Vec<usize> = self.page_extensions.get(&oi).cloned().unwrap_or_default();
        unsupported.extend(super::shared::over_max_length_of(self.ws, oi));
        for &e in &extensions {
            unsupported.extend(super::shared::over_max_length_of(self.ws, e));
        }
        // The code a control's text names lives in its owner: the page, or the extension. Each owner is built once per
        // page and shared by its controls (built per control, the global-name set and the syntax tree of an extension
        // cost a walk of the whole object for every control).
        struct Owner {
            object: usize,
            obj: ast::Object,
            names: std::collections::BTreeSet<String>,
            base: usize,
            /// The index of the first routine of `object` with each name, made when first asked for.
            routines: std::cell::OnceCell<HashMap<String, usize>>,
        }
        let owners: std::cell::RefCell<HashMap<Option<usize>, std::rc::Rc<Owner>>> = Default::default();
        let owner_of = |o: Option<usize>| -> std::rc::Rc<Owner> {
            owners
                .borrow_mut()
                .entry(o)
                .or_insert_with(|| {
                    let object = o.unwrap_or(oi);
                    let obj = if o.is_none() {
                        obj.clone()
                    } else {
                        self.ws.object_ast(object)
                    };
                    std::rc::Rc::new(Owner {
                        object,
                        names: crate::pages::global_names(&obj),
                        base: usize::from(obj.syntax().text_range().start()),
                        obj,
                        routines: std::cell::OnceCell::new(),
                    })
                })
                .clone()
        };
        let routine_named = |ow: &Owner, name: &str| {
            ow.routines
                .get_or_init(|| {
                    let mut map = HashMap::default();
                    for (i, r) in self.routine_nodes.get(ow.object).iter().enumerate() {
                        map.entry(r.name()).or_insert(i);
                    }
                    map
                })
                .get(name)
                .copied()
        };
        let routine_of = |ow: &Owner, r: &ast::Routine| {
            self.routine_at
                .get(&(ow.object, usize::from(r.syntax().text_range().start())))
        };
        // The routine behind an expression-valued property written at `p` in `ow`.
        let prop_value = |ow: &Owner,
                          p: &ast::Property,
                          unsupported: &mut Vec<String>,
                          big_integer: &mut Vec<String>,
                          control: &str,
                          kind: Option<ir::ControlKind>| {
            let name = p.name().to_ascii_lowercase();
            let value = p.value_text();
            let off = usize::from(p.syntax().text_range().start()) - ow.base;
            // An action's Visible or Enabled, or an action group's Visible, bound to a call of one of the page's own
            // procedures with no arguments compiles with warning AL0573 and reads No whatever the procedure returns; the
            // procedure never runs (pages/lt-qq-procedure-bound-action/QqBoundProps.al:29, :77-211, QQ_ProcBound_VisibleEnabled
            // and QQ_ProcBound_EditMode, on BC 27, 28 and 29). Only `kind` of the page's own controls is passed. A call with
            // arguments, inside a larger expression or of anything else is not pinned and stays refused below.
            let procedure_bound = match kind {
                Some(ir::ControlKind::Action) => name == "visible" || name == "enabled",
                Some(ir::ControlKind::ActionGroup) => name == "visible",
                _ => false,
            } && crate::pages::bare_call_name(&value).is_some_and(|f| {
                self.routine_nodes
                    .get(ow.object)
                    .iter()
                    .any(|r| r.name().trim_matches('"').eq_ignore_ascii_case(&f))
            });
            if procedure_bound {
                return (name, "false".to_string(), None);
            }
            let dynamic = crate::pages::dynamic_type(&name).filter(|t| !crate::pages::is_literal(*t, &value));
            // A Boolean control expression BC compiles on the client when the page opens (see `client_expression`).
            if dynamic == Some(crate::pages::PropType::Boolean) {
                match crate::pages::client_expression(&value) {
                    Some(crate::pages::ClientExpr::Refused(why)) => {
                        unsupported.push(format!("property {name} of control {control}: {why}"))
                    }
                    Some(crate::pages::ClientExpr::BigIntegerLiteral(why)) => {
                        big_integer.push(format!("property {name} of control {control}: {why}"))
                    }
                    None => {}
                }
            }
            let val = match dynamic {
                Some(_) => match routine_named(ow, &crate::pages::synthetic_name(&name, off)) {
                    Some(r) => Some(r),
                    None => {
                        unsupported.push(format!("property {name} of control {control} has no routine"));
                        None
                    }
                },
                None => None,
            };
            (name, value, val)
        };
        // The table's field names and the page's global names for `control_expr_text`, made once per page when a control
        // first needs them (made per control, they cost a walk of the whole page object for every control that sets one of
        // the properties).
        let expr_fields: std::cell::OnceCell<Vec<String>> = std::cell::OnceCell::new();
        let expr_globals: std::cell::OnceCell<Vec<String>> = std::cell::OnceCell::new();
        let mut controls = Vec::new();
        for raw in &layout.controls {
            let sec = &raw.section;
            // A data item of a request page: the test client reads and writes the filters of its
            // record. The item's own properties and triggers belong to the report run.
            if raw.kind == ir::ControlKind::DataItem {
                let binding = match self.globals.get(&raw.name.to_lowercase()) {
                    Some(slot) => ir::Binding::Var(*slot),
                    None => {
                        unsupported.push(format!("data item {} has no record", raw.name));
                        ir::Binding::None
                    }
                };
                controls.push(ir::Control {
                    name: raw.name.clone(),
                    kind: ir::ControlKind::DataItem,
                    parent: None,
                    binding,
                    props: Vec::new(),
                    triggers: Vec::new(),
                    part_page: None,
                    part_link: None,
                    owner: None,
                    ext_triggers: Vec::new(),
                    run_object: None,
                    action_ref: None,
                    prop_texts: Vec::new(),
                    var_field: None,
                    var_slot: None,
                    expr_source: None,
                    expr_call: false,
                });
                continue;
            }
            let ow = owner_of(raw.owner);
            let mut props: Vec<(String, ir::PropVal)> = Vec::new();
            for p in sec.properties() {
                let (name, value, routine) = prop_value(
                    &ow,
                    &p,
                    &mut unsupported,
                    &mut big_integer_expressions,
                    &raw.name,
                    raw.owner.is_none().then(|| raw.kind.clone()),
                );
                props.push((
                    name,
                    match routine {
                        Some(r) => ir::PropVal::Routine(r),
                        None => ir::PropVal::Lit(value),
                    },
                ));
            }
            // `modify` blocks replace what the control states, in the order the extensions apply. What each
            // block changes is also kept apart (`PageMods`), so that the symbolic form can leave it to link.
            let mut ext_triggers = Vec::new();
            let props_before = (!raw.modifies.is_empty()).then(|| props.clone());
            for (e, msec) in &raw.modifies {
                let mow = owner_of(Some(*e));
                let mut changed = Vec::new();
                let mut added = Vec::new();
                for p in msec.properties() {
                    let (name, value, routine) = prop_value(
                        &mow,
                        &p,
                        &mut unsupported,
                        &mut big_integer_expressions,
                        &raw.name,
                        None,
                    );
                    let val = match routine {
                        Some(r) => ir::PropVal::Ext(*e, r),
                        None => ir::PropVal::Lit(value),
                    };
                    changed.push((name.clone(), val.clone()));
                    match props.iter_mut().find(|(n, _)| *n == name) {
                        Some(slot) => slot.1 = val,
                        None => props.push((name, val)),
                    }
                }
                for t in msec.triggers() {
                    match routine_of(&mow, &t) {
                        Some(r) => {
                            ext_triggers.push((t.name().to_ascii_lowercase(), *e, r));
                            added.push((t.name().to_ascii_lowercase(), r));
                        }
                        None => {
                            unsupported.push(format!("trigger {} of control {} has no routine", t.name(), raw.name))
                        }
                    }
                }
                self.page_mods.borrow_mut().mods.push(super::localize::PageMod {
                    control: controls.len(),
                    ext: *e,
                    props: changed,
                    triggers: added,
                });
            }
            if let Some(before) = props_before {
                self.page_mods.borrow_mut().before.push((controls.len(), before));
            }
            // What Page Control Field reports for an expression-valued Visible, Enabled or Editable of the page's own control
            // that no extension changes (crate::pages::control_expr_text). A plain field or variable is no expression.
            let mut prop_texts = Vec::new();
            if raw.owner.is_none() {
                let changed: Vec<String> = raw
                    .modifies
                    .iter()
                    .flat_map(|(_, m)| m.properties().map(|p| p.name().to_ascii_lowercase()))
                    .collect();
                for p in sec.properties() {
                    let name = p.name().to_ascii_lowercase();
                    if !["visible", "enabled", "editable"].contains(&name.as_str()) || changed.contains(&name) {
                        continue;
                    }
                    let fields = expr_fields.get_or_init(|| {
                        table
                            .map(|t| self.members[t].fields.iter().map(|f| f.name.clone()).collect())
                            .unwrap_or_default()
                    });
                    let globals = expr_globals
                        .get_or_init(|| obj.var_decls().flat_map(|v| v.names()).map(|n| n.to_string()).collect());
                    if let Some(text) = obj
                        .id()
                        .and_then(|id| crate::pages::control_expr_text(&p.value_text(), id, globals, fields))
                    {
                        prop_texts.push((name, text));
                    }
                }
            }
            let mut binding = ir::Binding::None;
            let mut var_field = None;
            let mut var_slot = None;
            let mut expr_source = None;
            let mut expr_call = false;
            if raw.kind == ir::ControlKind::Field && sec.name().eq_ignore_ascii_case("label") {
                binding = ir::Binding::Caption;
            } else if raw.kind == ir::ControlKind::Field {
                let args = sec.args();
                // `args` unquotes a lone quoted name ("Profile Name" becomes Profile Name), which `plain_name` no longer
                // reads as a name: it reads the argument as written (al-runner issue 6314).
                let raw_args = sec.args_raw();
                match args.get(1).map(|a| crate::pages::classify_source(a, &ow.names)) {
                    Some(crate::pages::Source::RecField(f)) => {
                        let pos = table.and_then(|t| {
                            self.members[t]
                                .fields
                                .iter()
                                .position(|x| x.name.eq_ignore_ascii_case(&f))
                        });
                        match pos {
                            Some(p) => binding = ir::Binding::Field(p),
                            None => unsupported.push(format!("control {} is bound to unknown field {f}", raw.name)),
                        }
                    }
                    // A page variable with the name of a field of the source table: which one a bare name binds is not pinned.
                    Some(crate::pages::Source::Var(v))
                        if table.is_some_and(|t| {
                            self.members[t].fields.iter().any(|x| x.name.eq_ignore_ascii_case(&v))
                        }) =>
                    {
                        unsupported.push(format!(
                            "control {} is bound to the bare name {v}, which is a page variable and also a field of the source table (which one BC binds is not pinned)",
                            raw.name
                        ));
                    }
                    Some(crate::pages::Source::Var(v)) => {
                        let slot = match raw.owner {
                            None => globals.iter().position(|g| g.name.eq_ignore_ascii_case(&v)),
                            Some(e) => self.ext_slots[e].get(&v.to_lowercase()).copied(),
                        };
                        match slot {
                            Some(slot) => binding = ir::Binding::Var(slot),
                            None => unsupported.push(format!(
                                "control {} is bound to variable {v} of an unsupported type",
                                raw.name
                            )),
                        }
                    }
                    // A bare name of a field of the source table (no `Rec.`, warning AL0604) is that field: the control shows
                    // the field's caption and value (pages/lt-page-controls-aa/AaControls.al:684-692, `SystemId`). Pinned only where
                    // nothing else in scope has the name; with a procedure of the page or the table, a page variable or another
                    // control of that name, which one BC binds is not pinned: refused.
                    Some(crate::pages::Source::Expr)
                        if let Some((n, p)) = crate::pages::plain_name(raw_args.get(1).map_or("", |a| a.as_str()))
                            .and_then(|n| {
                                let t = table?;
                                let p = self.members[t]
                                    .fields
                                    .iter()
                                    .position(|x| x.name.eq_ignore_ascii_case(&n))?;
                                Some((n, p))
                            }) =>
                    {
                        let named = |o: usize| {
                            self.routine_nodes
                                .get(o)
                                .iter()
                                .any(|r| r.name().eq_ignore_ascii_case(&n))
                        };
                        let other = named(oi)
                            || named(ow.object)
                            || table.is_some_and(named)
                            || ow.names.contains(&n.to_lowercase())
                            || globals.iter().any(|g| g.name.eq_ignore_ascii_case(&n))
                            || layout
                                .controls
                                .iter()
                                .any(|c| !std::ptr::eq(c, raw) && c.name.eq_ignore_ascii_case(&n));
                        if other {
                            unsupported.push(format!(
                                "control {} is bound to the bare name {n}, which is a field of the source table and also another name in scope (which one BC binds is not pinned)",
                                raw.name
                            ));
                        } else {
                            binding = ir::Binding::Field(p);
                        }
                    }
                    Some(crate::pages::Source::Expr) => {
                        let off = usize::from(sec.syntax().text_range().start()) - ow.base;
                        match routine_named(&ow, &crate::pages::synthetic_name("source", off)) {
                            Some(r) => {
                                binding = ir::Binding::Expr(r);
                                // A field of a record variable of the page itself: the runtime formats the control from
                                // that field's properties (al-runner issue 5822).
                                if raw.owner.is_none() {
                                    let found =
                                        args.get(1)
                                            .and_then(|a| crate::pages::var_and_field(a))
                                            .and_then(|(v, f)| {
                                                let slot =
                                                    globals.iter().position(|g| g.name.eq_ignore_ascii_case(&v))?;
                                                let RtType::Record(t, _) = globals[slot].ty else {
                                                    return None;
                                                };
                                                let t = t as usize;
                                                let p = self.members[t]
                                                    .fields
                                                    .iter()
                                                    .position(|x| x.name.eq_ignore_ascii_case(&f))?;
                                                Some(((t, p), slot))
                                            });
                                    (var_field, var_slot) = (found.map(|f| f.0), found.map(|f| f.1));
                                    expr_call = args.get(1).is_some_and(|a| crate::pages::call_shape(a));
                                    // Only text with no quote and single spaces: how BC writes other text is not pinned.
                                    expr_source =
                                        args.get(1).filter(|a| crate::pages::plain_spacing(a)).and_then(|a| {
                                            let element = crate::pages::array_element(a).is_some_and(|(v, _)| {
                                                globals.iter().any(|g| {
                                                    g.name.eq_ignore_ascii_case(&v) && matches!(g.ty, RtType::Array(..))
                                                })
                                            });
                                            if element || found.is_some() {
                                                Some((a.trim().to_string(), true))
                                            } else {
                                                crate::pages::arithmetic(a).then(|| (a.trim().to_string(), false))
                                            }
                                        });
                                }
                            }
                            // A control of an extension that names a variable of the base object
                            // (`pages::needed` made no routine for it).
                            None => {
                                let base_slot = raw
                                    .owner
                                    .and_then(|_| crate::pages::plain_name(raw_args.get(1)?))
                                    .and_then(|n| globals.iter().position(|g| g.name.eq_ignore_ascii_case(&n)));
                                match base_slot {
                                    Some(slot) => binding = ir::Binding::BaseVar(slot),
                                    None => unsupported.push(format!(
                                        "control {} is bound to a name that is no variable of its base object",
                                        raw.name
                                    )),
                                }
                            }
                        }
                    }
                    None => {}
                }
            }
            let mut triggers: Vec<(String, usize)> = sec
                .triggers()
                .filter_map(|t| routine_of(&ow, &t).map(|r| (t.name().to_ascii_lowercase(), r)))
                .collect();
            // The routine that writes an expression source (`pages::expand` made it): kept with the triggers under a name
            // no AL trigger has.
            if matches!(binding, ir::Binding::Expr(_)) {
                let off = usize::from(sec.syntax().text_range().start()) - ow.base;
                if let Some(r) = routine_named(&ow, &crate::pages::synthetic_name("sourceset", off)) {
                    triggers.push(("__sourceset".into(), r));
                }
            }
            let file = self.ws.objects[ow.object].file;
            let part_page = if raw.kind == ir::ControlKind::Part {
                match crate::types::section_object_name(sec, 1).map(|n| self.resolve_obj_in(file, ObjKind::Page, &n)) {
                    Some(Ok(p)) => Some(p),
                    _ => {
                        unsupported.push(format!("part {} shows a page outside the workspace", raw.name));
                        None
                    }
                }
            } else {
                None
            };
            let part_link = match (part_page, raw.kind == ir::ControlKind::Part) {
                (Some(pp), true) => match self.part_link(oi, file, layout, sec, pp) {
                    Ok(l) => Some(l),
                    Err(why) => {
                        unsupported.push(format!("part {}: {why}", raw.name));
                        None
                    }
                },
                _ => None,
            };
            let action_ref = if sec.name().eq_ignore_ascii_case("actionref") {
                Some(match sec.args().get(1) {
                    None => ir::ActionRef::Unresolved("an actionref names no action".into()),
                    Some(t) => match layout.controls.iter().position(|c| {
                        c.kind == ir::ControlKind::Action
                            && !c.section.name().eq_ignore_ascii_case("actionref")
                            && c.name.eq_ignore_ascii_case(t.trim().trim_matches('"'))
                    }) {
                        Some(i) => ir::ActionRef::Target(i),
                        None => {
                            ir::ActionRef::Unresolved(format!("actionref to `{t}`, which is not an action of the page"))
                        }
                    },
                })
            } else {
                None
            };
            // An action that declares both RunObject and OnAction runs only the RunObject
            // (handlers/TestPageRunObjectContracts.al:371-390).
            let run_object = self.run_object(oi, file, sec);
            controls.push(ir::Control {
                name: raw.name.clone(),
                kind: raw.kind.clone(),
                parent: raw.parent,
                binding,
                props,
                triggers,
                part_page,
                part_link,
                owner: raw.owner,
                ext_triggers,
                run_object,
                action_ref,
                prop_texts,
                var_field,
                var_slot,
                expr_source,
                expr_call,
            });
        }
        let base = owner_of(None);
        let triggers = match request_section {
            Some(sec) => sec
                .triggers()
                .filter_map(|t| routine_of(&base, &t).map(|r| (t.name().to_ascii_lowercase(), r)))
                .collect(),
            None if obj.kind().eq_ignore_ascii_case("report") => Vec::new(),
            None => obj
                .triggers()
                .filter_map(|t| routine_of(&base, &t).map(|r| (t.name().to_ascii_lowercase(), r)))
                .collect(),
        };
        let mut ext_triggers = Vec::new();
        for &e in &extensions {
            let ow = owner_of(Some(e));
            // The triggers of a reportextension's request page are in its `requestpage` section.
            let triggers: Vec<ast::Routine> = if request_section.is_some() || obj.kind().eq_ignore_ascii_case("report")
            {
                ow.obj
                    .sections()
                    .filter(|s| s.name().eq_ignore_ascii_case("requestpage"))
                    .flat_map(|s| s.triggers().collect::<Vec<_>>())
                    .collect()
            } else {
                ow.obj.triggers().collect()
            };
            for t in triggers {
                if let Some(r) = routine_of(&ow, &t) {
                    ext_triggers.push((t.name().to_ascii_lowercase(), e, r));
                }
            }
        }
        ir::Page {
            controls,
            triggers,
            curr_page_slot: globals.iter().position(|g| g.name == "CurrPage").unwrap_or(0),
            unsupported,
            extensions,
            ext_triggers,
            action_areas: layout.action_areas.clone(),
            assumes: layout.assumes.clone(),
            big_integer_expressions,
        }
    }

    /// The `RunObject` of action `sec` of page `oi` with its `RunPageLink`, `RunPageView`,
    /// `RunPageMode` and `RunPageOnRec`; `None` for an action without one. What the test client
    /// cannot follow yet becomes `unsupported` (the action is refused when it is invoked).
    pub(crate) fn run_object(&self, oi: usize, file: usize, sec: &ast::Section) -> Option<ir::RunObject> {
        let prop = |n: &str| {
            sec.properties()
                .find(|p| p.name().eq_ignore_ascii_case(n))
                .map(|p| p.value_text())
        };
        let text = prop("RunObject")?;
        let mut out = ir::RunObject {
            kind: ir::RunKind::Page,
            object: None,
            on_rec: false,
            mode: ir::RunMode::Default,
            link: None,
            unsupported: None,
        };
        let (kind, parts) = match crate::pages::parse_run_object(&text) {
            Ok(k) => k,
            Err(why) => {
                out.unsupported = Some(why);
                return Some(out);
            }
        };
        out.kind = kind;
        match prop("RunPageOnRec").map(|v| v.trim().to_ascii_lowercase()) {
            None => {}
            Some(v) if v == "true" => out.on_rec = true,
            Some(v) if v == "false" => {}
            Some(v) => out.unsupported = Some(format!("RunPageOnRec = {v}")),
        }
        match prop("RunPageMode").map(|v| v.trim().to_ascii_lowercase()) {
            None => {}
            Some(v) if v == "view" => out.mode = ir::RunMode::View,
            Some(v) if v == "edit" => out.mode = ir::RunMode::Edit,
            Some(v) if v == "create" => out.mode = ir::RunMode::Create,
            Some(v) => out.unsupported = Some(format!("RunPageMode = {v}")),
        }
        // The test surface refuses a report, xmlport or query before it looks at the target
        // (corpus handlers/TestPageActionRunObjectKinds_Tests.al:RunObjectNamingAReportIsRefused...,
        // ...KindsPrecompiled_Tests.al: the same for objects outside the workspace), so none of
        // them needs resolving.
        let target = match kind {
            ir::RunKind::Page => ObjKind::Page,
            ir::RunKind::Codeunit => ObjKind::Codeunit,
            ir::RunKind::Report | ir::RunKind::XmlPort | ir::RunKind::Query => return Some(out),
        };
        match self.ws.resolve_ref(file, target, &parts) {
            Resolution::Found(t) => out.object = Some(t),
            _ => {
                out.unsupported = Some(format!("RunObject `{}` is outside the workspace", text.trim()));
                return Some(out);
            }
        }
        let link = prop("RunPageLink");
        let view = prop("RunPageView");
        if kind == ir::RunKind::Codeunit {
            if link.is_some() || view.is_some() {
                out.unsupported = Some("RunPageLink or RunPageView on a codeunit target".into());
            }
            return Some(out);
        }
        if link.is_some() || view.is_some() {
            let mut pl = ir::PartLink {
                view: view.map(|v| v.trim().to_string()),
                ..Default::default()
            };
            if let Some(link) = link {
                let entries = (|| {
                    let target_table = self.table_of[out.object.expect("resolved")]
                        .ok_or("RunPageLink on a target page without a source table")?;
                    self.link_entries("RunPageLink", &link, file, self.table_of[oi], target_table)
                })();
                pl.consts = crate::pages::parse_run_page_link_kinds(&link)
                    .map(|k| k.iter().enumerate().filter(|(_, e)| e.2).map(|(i, _)| i).collect())
                    .unwrap_or_default();
                match entries {
                    Ok(e) => pl.entries = e,
                    Err(why) => out.unsupported = Some(why),
                }
            }
            out.link = Some(pl);
        }
        Some(out)
    }

    /// The entries of a `SubPageLink` or `RunPageLink` text: `what` names the property in errors,
    /// `host_table` is the table `field(...)` entries read, `target_table` the linked page's.
    pub(crate) fn link_entries(
        &self,
        what: &str,
        text: &str,
        file: usize,
        host_table: Option<usize>,
        target_table: usize,
    ) -> L<Vec<ir::LinkEntry>> {
        use crate::pages::LinkRhs;
        let field_pos = |t: usize, name: &str| {
            self.members[t]
                .fields
                .iter()
                .position(|x| x.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("{what} names field `{name}`, which table does not have"))
        };
        let mut entries = Vec::new();
        for (left, rhs) in crate::pages::parse_run_page_link(text).map_err(|e| e.replace("RunPageLink", what))? {
            let pf = field_pos(target_table, &left)?;
            entries.push(match rhs {
                LinkRhs::Field(f) => {
                    let ht = host_table
                        .ok_or_else(|| format!("{what} reads a host field, but the host has no source table"))?;
                    ir::LinkEntry::Field(pf, field_pos(ht, &f)?)
                }
                // An empty `const('')` selects the blank value, as in a where() (corpus record/lt-relation-where-empty-term/
                // RelationWhereEmptyTerm.al); Production BOM's `"Version Code" = const('')` (Tests-SCM-Manufacturing).
                LinkRhs::Filter(t) if t.is_empty() => ir::LinkEntry::Filter(pf, "''".to_string()),
                LinkRhs::Filter(t) => ir::LinkEntry::Filter(pf, t),
                // A qualified enum value is the member of the field's enum (pages/lt-uj-outside-leads/UjSubPageLink.al
                // UJ_SubPageLink_ConstEnum_Rows, corpus branch al-runtime-rs-staging-uj 1dc037a8: the field may share the enum's
                // name). The checker rejects a member the enum lacks (AL0132, must-not-compile 2276); an enum other than the
                // field's is not pinned.
                LinkRhs::EnumValue(enum_name, member) => {
                    if what != "SubPageLink" {
                        return Err(format!(
                            "{what} with a qualified enum value `{enum_name}::{member}` (not pinned)"
                        ));
                    }
                    let field = &self.members[target_table].fields[pf];
                    let Ty::Enum(field_enum) = &field.ty else {
                        return Err(format!(
                            "{what}: a qualified enum value on field `{}`, which is no Enum (not pinned)",
                            field.name
                        ));
                    };
                    let e = self.resolve_obj_in(file, ObjKind::Enum, &enum_name)?;
                    if self.resolve_obj_in(file, ObjKind::Enum, field_enum).ok() != Some(e) {
                        return Err(format!(
                            "{what}: the enum value `{enum_name}::{member}` on field `{}` of another enum (not pinned)",
                            field.name
                        ));
                    }
                    let Some((_, name)) = self.members[e]
                        .enum_values
                        .iter()
                        .find(|(_, n)| n.eq_ignore_ascii_case(&member))
                    else {
                        return Err(format!("{what}: `{enum_name}` has no value `{member}`"));
                    };
                    ir::LinkEntry::Filter(pf, name.clone())
                }
                LinkRhs::TableId(name) => {
                    let t = self.resolve_obj_in(file, ObjKind::Table, &name)?;
                    let id = self.ws.objects[t].id.ok_or("table without an id")?;
                    ir::LinkEntry::Filter(pf, id.to_string())
                }
            });
        }
        Ok(entries)
    }

    /// `SubPageLink`, `SubPageView` and `Provider` of the part control `sec` of page `oi`, which
    /// shows page `part_page`. `Err` is why the link cannot be followed yet.
    pub(crate) fn part_link(
        &self,
        oi: usize,
        file: usize,
        layout: &crate::pages::Layout,
        sec: &ast::Section,
        part_page: usize,
    ) -> L<ir::PartLink> {
        let prop = |n: &str| {
            sec.properties()
                .find(|p| p.name().eq_ignore_ascii_case(n))
                .map(|p| p.value_text())
        };
        let mut link = ir::PartLink::default();
        if let Some(pname) = prop("Provider") {
            let pname = pname.trim().trim_matches('"');
            let idx = layout
                .controls
                .iter()
                .position(|c| c.kind == ir::ControlKind::Part && c.name.eq_ignore_ascii_case(pname));
            link.provider = Some(idx.ok_or_else(|| format!("Provider `{pname}` is not a part of the page"))?);
        }
        link.view = prop("SubPageView").map(|v| v.trim().to_string());
        let Some(text) = prop("SubPageLink") else {
            return Ok(link);
        };
        // The host side: the page's own table, or the Provider part's table.
        let host_table = match link.provider {
            Some(pi) => {
                let Some(ppage) = layout.controls[pi]
                    .section
                    .args()
                    .get(1)
                    .and_then(|n| self.resolve_obj_in(file, ObjKind::Page, n).ok())
                else {
                    return Err("the Provider part shows a page outside the workspace".into());
                };
                self.table_of[ppage]
            }
            None => self.table_of[oi],
        };
        let part_table = self.table_of[part_page].ok_or("the part page has no source table to link")?;
        link.entries = self.link_entries("SubPageLink", &text, file, host_table, part_table)?;
        link.enum_values = crate::pages::parse_sub_page_link(&text)
            .map(|k| {
                k.iter()
                    .enumerate()
                    .filter(|(_, e)| matches!(e.1, crate::pages::LinkRhs::EnumValue(..)))
                    .map(|(i, _)| i)
                    .collect()
            })
            .unwrap_or_default();
        Ok(link)
    }
}
