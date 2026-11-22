//! The text of an object as a rule pinned for ONE outside reporter's repro compares it: the page's properties, its controls
//! with their properties and triggers, its own triggers, and for a table its fields, keys and code, each line written in a
//! plain AL-like form (`pinned_body::page_lines`, `table_lines`, `routine_text`). The pin states the repro's lines and requires
//! them all, in order, with equality (al-runner PR 7838 review, round 2: an entry condition that checks only names or a
//! subset also runs on pages the repro does not show). Anything the renderer does not know gives `None`: the object is not
//! the pinned one. A pinned line may hold a name slot (`«T»`, see [`Names`]) where the pins measured the same answer under
//! other object and variable names; every other character is compared as written.

use al_builtins::value::RtType;
use al_ir as ir;

/// The calls a rendered body makes to procedures (object, routine): the pin compares their bodies too (`routine_text`).
pub(super) type Callees = Vec<(usize, usize)>;

/// The lines of page `object`: its properties as written (`PageType = Worksheet`; SourceTable as the name of the table it
/// resolves to, in quotes), then each control
/// (`field Quantity in Rows = Quantity {applicationarea=All}` and a line per trigger; a part in the FactBoxes area is a
/// `factbox part`), then each page trigger
/// (`trigger OnOpenPage: begin ... end`). `None` for a page with an extension, an extension trigger, a refusal, a control
/// declared by an extension, or anything else the renderer does not write.
pub(super) fn page_lines(program: &ir::Program, object: usize, callees: &mut Callees) -> Option<Vec<String>> {
    let o = &program.objects[object];
    let page = o.page.as_ref()?;
    if !page.extensions.is_empty() || !page.ext_triggers.is_empty() || !page.unsupported.is_empty() {
        return None;
    }
    let table_object = o.rec_slot.and_then(|s| match o.globals.get(s)?.ty {
        RtType::Record(t, _) => program.objects.get(t as usize),
        _ => None,
    });
    let table = table_object.and_then(|t| t.table.as_ref());
    // SourceTable by the table it names, as written in quotes: the spelling in the source does not change the page.
    let mut lines: Vec<String> = o
        .properties
        .iter()
        .map(|(k, v)| match table_object {
            Some(t) if k.eq_ignore_ascii_case("sourcetable") => format!("{k} = \"{}\"", t.name),
            _ => format!("{k} = {v}"),
        })
        .collect();
    for c in &page.controls {
        // A control bound to an expression (`CustomValue[1]`, `TempKind.Name`), and a property written as an expression, are
        // written as the routine the lowering made of them (al-runner issue 8434).
        let expr_bound = matches!(c.binding, ir::Binding::Expr(_));
        let routine_props = c.prop_texts.iter().all(|(k, _)| {
            c.props
                .iter()
                .any(|(pk, v)| pk == k && matches!(v, ir::PropVal::Routine(_)))
        });
        let plain = c.owner.is_none()
            && c.ext_triggers.is_empty()
            && c.run_object.is_none()
            && routine_props
            && (expr_bound
                || (c.var_field.is_none()
                    && c.var_slot.is_none()
                    && c.expr_source.is_none()
                    && c.array_element.is_none()
                    && c.source_text.is_none()))
            && !c.expr_call
            && !c.bare_quoted_ext_source
            && (!c.factbox || c.kind == ir::ControlKind::Part);
        if !plain {
            return None;
        }
        let kind = match (&c.kind, &c.action_ref) {
            (_, Some(ir::ActionRef::Target(_))) => "actionref",
            (_, Some(ir::ActionRef::Unresolved(_))) => return None,
            (ir::ControlKind::Field, None) => "field",
            (ir::ControlKind::Group, None) => "group",
            (ir::ControlKind::Repeater, None) => "repeater",
            (ir::ControlKind::Action, None) => "action",
            (ir::ControlKind::ActionGroup, None) => "actiongroup",
            (ir::ControlKind::CueGroup, None) => "cuegroup",
            (ir::ControlKind::Part, None) if c.factbox => "factbox part",
            (ir::ControlKind::Part, None) => "part",
            _ => return None,
        };
        let parent = match c.parent {
            None => "-".to_string(),
            Some(p) => page.controls.get(p)?.name.clone(),
        };
        let binding = match c.binding {
            ir::Binding::None => match &c.action_ref {
                Some(ir::ActionRef::Target(t)) => format!(" -> {}", page.controls.get(*t)?.name),
                _ => String::new(),
            },
            ir::Binding::Field(f) => format!(" = {}", table?.fields.get(f)?.name),
            // A global variable of the page (`field(CurrentBatch; CurrentBatch)`), with its type.
            ir::Binding::Var(g) => {
                let v = o.globals.get(g)?;
                format!(" = var {}: {}", v.name, type_text(program, &v.ty)?)
            }
            ir::Binding::Expr(r) => format!(" = expr {}", routine_text(program, object, r, callees)?),
            _ => return None,
        };
        let mut props = Vec::new();
        for (k, v) in &c.props {
            match v {
                ir::PropVal::Lit(v) => props.push(format!("{k}={v}")),
                ir::PropVal::Routine(r) => {
                    props.push(format!("{k}=expr {}", routine_text(program, object, *r, callees)?))
                }
                ir::PropVal::Ext(..) => return None,
            }
        }
        let part =
            match (c.part_page, &c.part_link) {
                (None, None) => String::new(),
                (Some(p), link) => {
                    let mut s = format!(" page {}", program.objects.get(p)?.name);
                    if let Some(l) = link {
                        let part_table = program.objects.get(p)?.rec_slot.and_then(|s| {
                            match program.objects[p].globals.get(s)?.ty {
                                RtType::Record(t, _) => program.objects.get(t as usize)?.table.as_ref(),
                                _ => None,
                            }
                        })?;
                        if l.view.is_some() || l.provider.is_some() || !l.consts.is_empty() || !l.enum_values.is_empty()
                        {
                            return None;
                        }
                        for e in &l.entries {
                            let ir::LinkEntry::Field(pf, hf) = e else { return None };
                            s += &format!(
                                " link {} = field({})",
                                part_table.fields.get(*pf)?.name,
                                table?.fields.get(*hf)?.name
                            );
                        }
                    }
                    s
                }
                (None, Some(_)) => return None,
            };
        lines.push(format!(
            "{kind} {} in {parent}{binding}{part} {{{}}}",
            c.name,
            props.join("; ")
        ));
        for (t, r) in &c.triggers {
            lines.push(format!(
                "  trigger {t}: {}",
                routine_text(program, object, *r, callees)?
            ));
        }
    }
    for a in &page.action_areas {
        let roots: Option<Vec<&str>> = a
            .roots
            .iter()
            .map(|r| Some(page.controls.get(*r)?.name.as_str()))
            .collect();
        lines.push(format!("area {}: {}", a.name, roots?.join(", ")));
    }
    for (t, r) in &page.triggers {
        lines.push(format!("trigger {t}: {}", routine_text(program, object, *r, callees)?));
    }
    Some(lines)
}

/// The lines of table `object`: its properties, each field (`field 2 "Line No.": Integer`), each key (`key "Batch Name",
/// "Line No."`) and each routine (`trigger OnModify: ...`, `trigger "Name".OnValidate: ...`, `procedure X: ...`); an
/// AutoIncrement field ends in ` AutoIncrement`, a field with a TableRelation in its properties (`{NotBlank=true; ...}`).
/// `None` for a table with an extension or a refusal, or a field with an initial value.
pub(super) fn table_lines(program: &ir::Program, object: usize, callees: &mut Callees) -> Option<Vec<String>> {
    let o = &program.objects[object];
    let t = o.table.as_ref()?;
    if !t.extensions.is_empty() || !t.unsupported.is_empty() {
        return None;
    }
    let mut lines: Vec<String> = o.properties.iter().map(|(k, v)| format!("{k} = {v}")).collect();
    for f in &t.fields {
        // The platform's own fields (SystemId and the audit fields) every table has.
        if f.id >= 2_000_000_000 {
            continue;
        }
        if f.init.is_some() || !f.stored {
            return None;
        }
        let auto = if f.auto_increment { " AutoIncrement" } else { "" };
        // A field with a TableRelation is written with its properties as written (al-runner issue 8434).
        let relation = f.relation || f.relation_table.is_some() || !f.relation_tables.is_empty();
        let props = if relation {
            let props: Vec<String> = f
                .properties
                .iter()
                .filter(|(k, _)| !k.eq_ignore_ascii_case("autoincrement"))
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            format!(" {{{}}}", props.join("; "))
        } else {
            String::new()
        };
        lines.push(format!(
            "field {} \"{}\": {}{auto}{props}",
            f.id,
            f.name,
            type_text(program, &f.ty)?
        ));
    }
    let no_props = Vec::new();
    for (i, k) in t.keys.iter().enumerate() {
        let props: Vec<String> = t
            .key_props
            .get(i)
            .unwrap_or(&no_props)
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        let names: Option<Vec<String>> = k
            .iter()
            .map(|f| Some(format!("\"{}\"", t.fields.get(*f)?.name)))
            .collect();
        lines.push(format!("key {} {{{}}}", names?.join(", "), props.join("; ")));
    }
    for (r, routine) in o.routines.iter().enumerate() {
        let kind = if routine.is_trigger { "trigger" } else { "procedure" };
        // A field's trigger names its field (`trigger "Name".OnValidate`).
        let field = t
            .fields
            .iter()
            .find(|f| f.on_validate == Some((object, r)) || f.on_lookup == Some((object, r)));
        let name = match field {
            Some(f) => format!("\"{}\".{}", f.name, routine.name),
            None => routine.name.clone(),
        };
        lines.push(format!("{kind} {name}: {}", routine_text(program, object, r, callees)?));
    }
    Some(lines)
}

/// The lines of the object-level code of page, table or codeunit `object` that `page_lines` and `table_lines` leave out
/// (al-runner issue 8434): each global variable but Rec, xRec and CurrPage (`var Setup: Record "LT NF Setup"`), then each
/// procedure with its signature (`local procedure RefreshUsage(): begin ... end`, `procedure Add(Entry: Text): ...`); for
/// a codeunit, also its properties first and its triggers (`trigger OnRun: ...`). The routines the lowering makes of
/// control expressions (`__editable_..`) are written with their controls (`page_lines`), and a table's and page's
/// triggers by `table_lines` and `page_lines`. `None` for anything the renderer does not write.
pub(super) fn object_code_lines(program: &ir::Program, object: usize, callees: &mut Callees) -> Option<Vec<String>> {
    let o = &program.objects[object];
    let codeunit = o.table.is_none() && o.page.is_none();
    let mut lines: Vec<String> = if codeunit {
        o.properties.iter().map(|(k, v)| format!("{k} = {v}")).collect()
    } else {
        Vec::new()
    };
    let curr_page = o.page.as_ref().map(|p| p.curr_page_slot);
    for (g, slot) in o.globals.iter().enumerate() {
        if Some(g) == o.rec_slot || Some(g) == o.xrec_slot || Some(g) == curr_page {
            continue;
        }
        lines.push(format!("var {}: {}", slot.name, type_text(program, &slot.ty)?));
    }
    for (r, routine) in o.routines.iter().enumerate() {
        if routine.name.starts_with("__") || (routine.is_trigger && !codeunit) || o.table.is_some() {
            continue;
        }
        let kind = match (routine.is_trigger, routine.local) {
            (true, _) => "trigger",
            (false, true) => "local procedure",
            (false, false) => "procedure",
        };
        let params: Option<Vec<String>> = (0..routine.params)
            .map(|i| {
                let p = routine.locals.get(i)?;
                let var = if routine.by_var.get(i).copied().unwrap_or(false) {
                    "var "
                } else {
                    ""
                };
                Some(format!("{var}{}: {}", p.name, type_text(program, &p.ty)?))
            })
            .collect();
        let ret = match &routine.ret {
            None => String::new(),
            Some(t) => format!(": {}", type_text(program, t)?),
        };
        lines.push(format!(
            "{kind} {}({}){ret}: {}",
            routine.name,
            params?.join("; "),
            routine_text(program, object, r, callees)?
        ));
    }
    Some(lines)
}

/// Whether table `object` reads, line for line, as `want` (`table_lines`); its subscribers are the caller's to check.
pub(super) fn table_reads_as(
    program: &ir::Program,
    object: usize,
    want: &[&str],
    callees: &mut Callees,
    names: &mut Names,
) -> bool {
    table_lines(program, object, callees).is_some_and(|have| names.lines_are(&have, want))
}

/// Whether the object-level code of `object` reads, line for line, as `want` (`object_code_lines`).
pub(super) fn object_code_reads_as(
    program: &ir::Program,
    object: usize,
    want: &[&str],
    callees: &mut Callees,
    names: &mut Names,
) -> bool {
    object_code_lines(program, object, callees).is_some_and(|have| names.lines_are(&have, want))
}

/// The body of routine `r` of `object`: `var Posted: Record "Journal Post Line"; begin Posted.Copy(Rec); end`, every
/// expression of more than one operand in parentheses, a field number argument (`SetRange`'s) as `field#<position>`. The procedures it calls are added to `callees`. `None` when it
/// holds a statement or an expression the renderer does not write (a loop, a `case`, a try call, ...).
pub(super) fn routine_text(program: &ir::Program, object: usize, r: usize, callees: &mut Callees) -> Option<String> {
    let routine = program.objects[object].routines.get(r)?;
    let w = Writer {
        program,
        object,
        routine,
    };
    let mut vars = Vec::new();
    for (i, l) in routine.locals.iter().enumerate().skip(routine.params) {
        if Some(i) == routine.ret_local {
            continue;
        }
        vars.push(format!("{}: {}", l.name, type_text(program, &l.ty)?));
    }
    let mut s = String::new();
    if !vars.is_empty() {
        s = format!("var {}; ", vars.join("; "));
    }
    s += "begin ";
    s += &w.stmts(routine.body.stmts(), callees)?;
    s += "end";
    Some(s)
}

/// The names the name slots of pinned lines stand for, once bound: the same slot stands for the same name in every line of
/// one comparison. Slots: `«V»` a variable name (letters, digits, underscore); any other letter (`«T»`, `«L»`, ...) an object
/// name or an object id (anything but a double quote).
#[derive(Debug, Default)]
pub(super) struct Names(Vec<(char, String)>);

impl Names {
    /// Whether `have` reads as `want`, the name slots of `want` bound as `self` holds them or bound now.
    pub(super) fn line_is(&mut self, have: &str, want: &str) -> bool {
        let before = self.0.len();
        let ok = self.rest_is(have, want);
        if !ok {
            self.0.truncate(before);
        }
        ok
    }

    fn rest_is(&mut self, have: &str, want: &str) -> bool {
        let Some(at) = want.find('«') else {
            return have == want;
        };
        let (lit, slot) = want.split_at(at);
        let mut chars = slot.chars();
        let (Some('«'), Some(name), Some('»')) = (chars.next(), chars.next(), chars.next()) else {
            return false;
        };
        let rest = chars.as_str();
        let Some(have) = have.strip_prefix(lit) else {
            return false;
        };
        if let Some((_, bound)) = self.0.iter().find(|(n, _)| *n == name) {
            let bound = bound.clone();
            return have.strip_prefix(bound.as_str()).is_some_and(|h| self.rest_is(h, rest));
        }
        let allowed = |c: char| match name {
            'V' => c.is_ascii_alphanumeric() || c == '_',
            _ => c != '"',
        };
        for (end, c) in have.char_indices() {
            if !allowed(c) {
                break;
            }
            let end = end + c.len_utf8();
            self.0.push((name, have[..end].to_string()));
            if self.rest_is(&have[end..], rest) {
                return true;
            }
            self.0.pop();
        }
        false
    }

    /// The name slot `slot` stands for, once bound.
    pub(super) fn bound(&self, slot: char) -> Option<&str> {
        self.0.iter().find(|(n, _)| *n == slot).map(|(_, name)| name.as_str())
    }

    /// Whether `have` reads, line for line, as `want`; when not, nothing is bound.
    fn lines_are(&mut self, have: &[String], want: &[&str]) -> bool {
        let before = self.0.len();
        let ok = have.len() == want.len() && have.iter().zip(want).all(|(h, w)| self.line_is(h, w));
        if !ok {
            self.0.truncate(before);
        }
        ok
    }
}

/// Whether page `object` reads, line for line, as `want` (`page_lines`), and nothing subscribes to its events.
pub(super) fn page_is(
    engine: &crate::Engine,
    object: usize,
    want: &[&str],
    callees: &mut Callees,
    names: &mut Names,
) -> bool {
    !engine.events.any_subscriber_of(object) && page_reads_as(&engine.program, object, want, callees, names)
}

/// Whether page `object` reads, line for line, as `want` (`page_lines`); its subscribers are the caller's to check.
pub(super) fn page_reads_as(
    program: &ir::Program,
    object: usize,
    want: &[&str],
    callees: &mut Callees,
    names: &mut Names,
) -> bool {
    page_lines(program, object, callees).is_some_and(|have| names.lines_are(&have, want))
}

/// Whether the globals of report `object` after its data item records read, line for line, as `want`, each written
/// `var SomeFilter: Text`: the slots between the report's hidden request page flag (`ir::Report::request_flag_slot`) and the
/// request page's `CurrPage` (`ir::Page::curr_page_slot`), the declared globals and then the request page's own variables
/// (al-sema `lower/shared.rs` lays them out in that order).
pub(super) fn report_globals_are(program: &ir::Program, object: usize, want: &[&str], names: &mut Names) -> bool {
    let o = &program.objects[object];
    let (Some(r), Some(page)) = (o.report.as_ref(), o.page.as_ref()) else {
        return false;
    };
    let Some(declared) = o.globals.get(r.request_flag_slot + 1..page.curr_page_slot) else {
        return false;
    };
    let have: Option<Vec<String>> = declared
        .iter()
        .map(|g| Some(format!("var {}: {}", g.name, type_text(program, &g.ty)?)))
        .collect();
    have.is_some_and(|have| names.lines_are(&have, want))
}

/// Whether table `object` reads, line for line, as `want` (`table_lines`), and nothing subscribes to its events.
pub(super) fn table_is(
    engine: &crate::Engine,
    object: usize,
    want: &[&str],
    callees: &mut Callees,
    names: &mut Names,
) -> bool {
    !engine.events.any_subscriber_of(object)
        && table_lines(&engine.program, object, callees).is_some_and(|have| names.lines_are(&have, want))
}

/// Whether every procedure in `callees` reads as one of `want`, each written `"<object>".<procedure>: <body>`, calls
/// nothing itself, and its object has no subscriber.
pub(super) fn callees_are(engine: &crate::Engine, callees: &Callees, want: &[&str], names: &mut Names) -> bool {
    callees.iter().all(|&(o, r)| {
        let mut inner = Vec::new();
        callee_text(engine, o, r, &mut inner).is_some_and(|text| {
            inner.is_empty() && !engine.events.any_subscriber_of(o) && want.iter().any(|w| names.line_is(&text, w))
        })
    })
}

/// As `callees_are`, and a procedure may call others that read as one of `want` and call nothing themselves (the page's
/// ChangeView of pages/currpage/ListValidateFilterRefresh.al calls the log codeunit's FilterStep and UpdateStep).
pub(super) fn callees_with_calls_are(
    engine: &crate::Engine,
    callees: &Callees,
    want: &[&str],
    names: &mut Names,
) -> bool {
    callees.iter().all(|&(o, r)| {
        let mut inner = Vec::new();
        callee_text(engine, o, r, &mut inner)
            .is_some_and(|text| !engine.events.any_subscriber_of(o) && want.iter().any(|w| names.line_is(&text, w)))
            && callees_are(engine, &inner, want, names)
    })
}

/// Routine `r` of `o` written `"<object>".<procedure>: <body>`; the procedures it calls go to `inner`.
fn callee_text(engine: &crate::Engine, o: usize, r: usize, inner: &mut Callees) -> Option<String> {
    let body = routine_text(&engine.program, o, r, inner)?;
    Some(format!(
        "\"{}\".{}: {body}",
        engine.program.objects[o].name, engine.program.objects[o].routines[r].name
    ))
}

/// A type as a declaration writes it; `None` for one the pins do not use.
fn type_text(program: &ir::Program, ty: &RtType) -> Option<String> {
    Some(match ty {
        RtType::Boolean => "Boolean".into(),
        RtType::Integer => "Integer".into(),
        RtType::Decimal => "Decimal".into(),
        RtType::Text(None) => "Text".into(),
        RtType::Text(Some(n)) => format!("Text[{n}]"),
        RtType::Code(Some(n)) => format!("Code[{n}]"),
        RtType::Date => "Date".into(),
        RtType::Option(members) => format!("Option {}", members.join(",")),
        RtType::Dictionary(k, v) => format!("Dictionary of [{}, {}]", type_text(program, k)?, type_text(program, v)?),
        RtType::Record(t, false) => format!("Record \"{}\"", program.objects.get(*t as usize)?.name),
        RtType::Record(t, true) => format!("Record \"{}\" temporary", program.objects.get(*t as usize)?.name),
        RtType::Codeunit(c) => format!("Codeunit \"{}\"", program.objects.get(*c as usize)?.name),
        RtType::Variant => "Variant".into(),
        RtType::Library(k) => k.name().to_string(),
        RtType::Array(dims, t) => {
            let dims: Vec<String> = dims.iter().map(u32::to_string).collect();
            format!("array[{}] of {}", dims.join(", "), type_text(program, t)?)
        }
        _ => return None,
    })
}

struct Writer<'a> {
    program: &'a ir::Program,
    object: usize,
    routine: &'a ir::Routine,
}

impl Writer<'_> {
    fn stmts(&self, stmts: &[ir::Stmt], callees: &mut Callees) -> Option<String> {
        let mut s = String::new();
        for st in stmts {
            match st {
                ir::Stmt::Pos(_) | ir::Stmt::Assume(_) | ir::Stmt::TddMark(_) => continue,
                ir::Stmt::Expr(e) => s += &format!("{}; ", self.expr(e, callees)?),
                ir::Stmt::Assign(p, e) => s += &format!("{} := {}; ", self.place(p)?, self.expr(e, callees)?),
                ir::Stmt::Exit(None) => s += "exit; ",
                ir::Stmt::Exit(Some(e)) => s += &format!("exit({}); ", self.expr(e, callees)?),
                ir::Stmt::If(c, then, other) => {
                    s += &format!(
                        "if {} then begin {}end",
                        self.expr(c, callees)?,
                        self.stmts(then, callees)?
                    );
                    if !other.is_empty() {
                        s += &format!(" else begin {}end", self.stmts(other, callees)?);
                    }
                    s += "; ";
                }
                ir::Stmt::Foreach { var, collection, body } => {
                    s += &format!(
                        "foreach {} in {} do begin {}end; ",
                        self.place(var)?,
                        self.expr(collection, callees)?,
                        self.stmts(body, callees)?
                    );
                }
                ir::Stmt::Repeat(body, until) => {
                    s += &format!(
                        "repeat {}until {}; ",
                        self.stmts(body, callees)?,
                        self.expr(until, callees)?
                    );
                }
                ir::Stmt::Case {
                    scrutinee,
                    branches,
                    otherwise,
                } => {
                    s += &format!("case {} of ", self.expr(scrutinee, callees)?);
                    for (values, body) in branches {
                        let values: Option<Vec<String>> = values
                            .iter()
                            .map(|v| match v {
                                ir::SetElem::One(e) => self.expr(e, callees),
                                ir::SetElem::Range(..) => None,
                            })
                            .collect();
                        s += &format!("{}: begin {}end; ", values?.join(", "), self.stmts(body, callees)?);
                    }
                    if !otherwise.is_empty() {
                        s += &format!("else begin {}end; ", self.stmts(otherwise, callees)?);
                    }
                    s += "end; ";
                }
                _ => return None,
            }
        }
        Some(s)
    }

    fn slot(&self, p: &ir::Place) -> Option<&ir::Slot> {
        match p {
            ir::Place::Local(i) => self.routine.locals.get(*i),
            ir::Place::Global(g) => self.program.objects[self.object].globals.get(*g),
            _ => None,
        }
    }

    fn place(&self, p: &ir::Place) -> Option<String> {
        match p {
            ir::Place::Local(_) | ir::Place::Global(_) => Some(self.slot(p)?.name.clone()),
            ir::Place::Field(base, f) => {
                let RtType::Record(t, _) = self.slot(base)?.ty else {
                    return None;
                };
                let table = self.program.objects.get(t as usize)?.table.as_ref()?;
                Some(format!("{}.\"{}\"", self.place(base)?, table.fields.get(*f)?.name))
            }
            // An array element at a constant index (`CustomValue[1]`).
            ir::Place::Index(base, at) => {
                let at: Option<Vec<String>> = at
                    .iter()
                    .map(|e| match e {
                        ir::Expr::Const(ir::Const::Integer(i)) => Some(i.to_string()),
                        _ => None,
                    })
                    .collect();
                Some(format!("{}[{}]", self.place(base)?, at?.join(", ")))
            }
            _ => None,
        }
    }

    fn arg(&self, a: &ir::Arg, callees: &mut Callees) -> Option<String> {
        match a {
            ir::Arg::Value(e) => self.expr(e, callees),
            ir::Arg::Ref(p) => self.place(p),
        }
    }

    fn args(&self, args: &[ir::Arg], callees: &mut Callees) -> Option<String> {
        let parts: Option<Vec<String>> = args.iter().map(|a| self.arg(a, callees)).collect();
        Some(parts?.join(", "))
    }

    fn expr(&self, e: &ir::Expr, callees: &mut Callees) -> Option<String> {
        Some(match e {
            ir::Expr::Const(c) => match c {
                ir::Const::Boolean(b) => b.to_string(),
                ir::Const::Integer(i) => i.to_string(),
                ir::Const::Decimal(d) => d.clone(),
                ir::Const::Text(t) => format!("'{}'", t.replace('\'', "''")),
                ir::Const::FieldPos(f) => format!("field#{f}"),
                ir::Const::Option(i, members) => format!("option#{}", members.get(usize::try_from(*i).ok()?)?),
                ir::Const::Enum(e, v) => format!("enum#\"{}\"#{v}", self.program.objects.get(*e as usize)?.name),
                _ => return None,
            },
            ir::Expr::Load(p) => self.place(p)?,
            // A conversion the compiler inserts to the type a parameter or an operator wants.
            ir::Expr::Convert(_, inner) => self.expr(inner, callees)?,
            ir::Expr::Unary(ir::UnOp::Not, x) => format!("(not {})", self.expr(x, callees)?),
            ir::Expr::Unary(ir::UnOp::Neg, x) => format!("(- {})", self.expr(x, callees)?),
            ir::Expr::Binary(op, a, b) => {
                let op = match op {
                    ir::BinOp::Add => "+",
                    ir::BinOp::Sub => "-",
                    ir::BinOp::Mul => "*",
                    ir::BinOp::IntDiv => "div",
                    ir::BinOp::Eq => "=",
                    ir::BinOp::Ne => "<>",
                    ir::BinOp::Div => "/",
                    ir::BinOp::Mod => "mod",
                    ir::BinOp::Xor => "xor",
                    ir::BinOp::Lt => "<",
                    ir::BinOp::Le => "<=",
                    ir::BinOp::Gt => ">",
                    ir::BinOp::Ge => ">=",
                    ir::BinOp::And => "and",
                    ir::BinOp::Or => "or",
                    _ => return None,
                };
                format!("({} {op} {})", self.expr(a, callees)?, self.expr(b, callees)?)
            }
            // The error a control's setter raises for a value of another kind (the lowering writes it).
            ir::Expr::Fail { message, code } => format!("fail('{message}', '{code}')"),
            ir::Expr::Ternary(c, a, b) => format!(
                "({} ? {} : {})",
                self.expr(c, callees)?,
                self.expr(a, callees)?,
                self.expr(b, callees)?
            ),
            ir::Expr::Call {
                object,
                routine,
                instance,
                args,
                try_call: false,
            } => {
                callees.push((*object, *routine));
                let name = &self.program.objects.get(*object)?.routines.get(*routine)?.name;
                let on = match instance {
                    None => String::new(),
                    Some(i) => format!("{}.", self.expr(i, callees)?),
                };
                format!("{on}{name}({})", self.args(args, callees)?)
            }
            ir::Expr::Builtin { id, receiver, args, .. } => {
                let on = match receiver {
                    None => String::new(),
                    Some(r) => format!("{}.", self.arg(r, callees)?),
                };
                format!("{on}{}({})", id.method(), self.args(args, callees)?)
            }
            // `CurrPage.<Control>` in the page's own code.
            ir::Expr::Control {
                page,
                control,
                runtime: true,
            } => {
                let name = &self.program.objects[self.object]
                    .page
                    .as_ref()?
                    .controls
                    .get(*control)?
                    .name;
                format!("{}.{name}", self.expr(page, callees)?)
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_slot_binds_once_and_stands_for_the_same_name_after() {
        let mut names = Names::default();
        assert!(names.line_is("SourceTable = \"TP Line\"", "SourceTable = \"«T»\""));
        assert!(names.line_is("var Posted: Record \"TP Line\"", "var Posted: Record \"«T»\""));
        assert!(!names.line_is("var Posted: Record \"Other\"", "var Posted: Record \"«T»\""));
        assert!(names.line_is(
            "var Log: Codeunit \"TP Log\"; begin Log.Add('R'); end",
            "var «V»: Codeunit \"«L»\"; begin «V».Add('R'); end"
        ));
        assert!(!names.line_is(
            "var Other: Codeunit \"TP Log\"; begin Other.Add('R'); end",
            "var «V»: Codeunit \"«L»\"; begin «V».Add('R'); end"
        ));
        // A failed line binds nothing: `«V»` still stands for `Log`, and `«T»` for `TP Line`.
        let mut fresh = Names::default();
        assert!(!fresh.line_is("x \"A\" y", "x \"«T»\" z"));
        assert!(fresh.line_is("x \"B\" z", "x \"«T»\" z"));
        // A variable slot takes no space or quote; a line without slots compares as written.
        assert!(!Names::default().line_is("var A B: Integer", "var «V»: Integer"));
        assert!(Names::default().line_is("{applicationarea=All}", "{applicationarea=All}"));
        assert!(!Names::default().line_is("{applicationarea=All}", "{applicationarea=all}"));
    }

    #[test]
    fn a_page_and_its_table_render_as_written() {
        let src = r#"
table 50760 "Journal Post Line"
{
    fields
    {
        field(1; "Batch Name"; Code[10]) { }
        field(2; "Line No."; Integer) { }
    }
    keys
    {
        key(PK; "Batch Name", "Line No.") { Clustered = true; }
    }
}
codeunit 50761 "Journal Post Log"
{
    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;
}
page 50762 "Journal Post New Row Wks"
{
    PageType = Worksheet;
    SourceTable = "Journal Post Line";
    AutoSplitKey = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(Post)
            {
                Caption = 'Post';
                trigger OnAction()
                var
                    Posted: Record "Journal Post Line";
                begin
                    Posted.Copy(Rec);
                    if not Posted.Find('=><') then begin
                        Posted.FilterGroup(2);
                        Posted.SetRange("Batch Name", 'B1');
                        Posted."Line No." := 1;
                    end;
                    CurrPage.Update(false);
                end;
            }
        }
    }
    trigger OnAfterGetRecord()
    var
        JournalPostLog: Codeunit "Journal Post Log";
    begin
        JournalPostLog.Add('R:' + Format(Rec."Line No." div 10000));
    end;
}
"#;
        let l = al_testkit::compile(src);
        let p = &l.program;
        let at = |name: &str| p.objects.iter().position(|o| o.name == name).unwrap();
        let mut callees = Vec::new();
        let page = page_lines(p, at("Journal Post New Row Wks"), &mut callees).unwrap();
        assert_eq!(
            page,
            [
                "PageType = Worksheet",
                "SourceTable = \"Journal Post Line\"",
                "AutoSplitKey = true",
                "repeater Rows in - {}",
                "field Line No. in Rows = Line No. {applicationarea=All}",
                "action Post in - {caption='Post'}",
                "  trigger onaction: var Posted: Record \"Journal Post Line\"; begin Posted.Copy(Rec); if (not \
                 Posted.Find('=><')) then begin Posted.FilterGroup(2); Posted.SetRange(field#0, 'B1'); Posted.\"Line No.\" := 1; \
                 end; CurrPage.Update(false); end",
                "area Processing: Post",
                "trigger onaftergetrecord: var JournalPostLog: Codeunit \"Journal Post Log\"; begin \
                 JournalPostLog.Add(('R:' + Format((Rec.\"Line No.\" div 10000)))); end",
            ]
        );
        let log = at("Journal Post Log");
        assert_eq!(callees, [(log, 0)]);
        assert_eq!(
            routine_text(p, log, 0, &mut Vec::new()).unwrap(),
            "begin Entries := (Entries + (Entry + ';')); end"
        );
        assert_eq!(
            table_lines(p, at("Journal Post Line"), &mut Vec::new()).unwrap(),
            [
                "field 1 \"Batch Name\": Code[10]",
                "field 2 \"Line No.\": Integer",
                "key \"Batch Name\", \"Line No.\" {Clustered=true}"
            ]
        );
    }

    #[test]
    fn a_control_bound_to_a_variable_and_a_case_render_as_written() {
        let src = r#"
table 50763 "Case Line"
{
    fields { field(1; "No."; Integer) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
page 50764 "Case Page"
{
    PageType = List;
    SourceTable = "Case Line";
    layout
    {
        area(Content)
        {
            field(Mode; Mode)
            {
                trigger OnValidate()
                begin
                    case Mode of
                        1, 2:
                            CurrPage.Update(true);
                        3:
                            Mode := 0;
                        else
                            Error('x');
                    end;
                end;
            }
        }
    }
    var
        Mode: Integer;
}
"#;
        let l = al_testkit::compile(src);
        let p = &l.program;
        let at = |name: &str| p.objects.iter().position(|o| o.name == name).unwrap();
        assert_eq!(
            page_lines(p, at("Case Page"), &mut Vec::new()).unwrap(),
            [
                "PageType = List",
                "SourceTable = \"Case Line\"",
                "field Mode in - = var Mode: Integer {}",
                "  trigger onvalidate: begin case Mode of 1, 2: begin CurrPage.Update(true); end; 3: begin Mode := 0; end; \
                 else begin Error('x'); end; end; end",
            ]
        );
    }

    #[test]
    fn loops_comparisons_and_variable_controls_render() {
        let src = r#"
table 50770 "Render Line"
{
    TableType = Temporary;
    fields
    {
        field(1; Kind; Option) { OptionMembers = Day,Week; }
        field(2; Day; Date) { }
        field(3; Qty; Decimal) { }
    }
    keys
    {
        key(PK; Kind, Day) { Clustered = true; }
    }
}
page 50771 "Render Part"
{
    PageType = ListPart;
    SourceTable = "Render Line";
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Qty; Rec.Qty) { StyleExpr = Style; }
                field(Shown; Shown) { }
            }
        }
    }
    var
        Shown: Text;
        Style: Text;
        Loaded: Boolean;

    procedure Fill(Values: Dictionary of [Text, Text])
    var
        DayKey: Text;
    begin
        foreach DayKey in Values.Keys() do begin
            Rec.Kind := Rec.Kind::Week;
            Rec.Insert();
        end;
        repeat
            Loaded := Loaded and (Rec.Qty >= 2);
        until Rec.Next() = 0;
    end;
}
"#;
        let l = al_testkit::compile(src);
        let p = &l.program;
        let at = |name: &str| p.objects.iter().position(|o| o.name == name).unwrap();
        let part = at("Render Part");
        let lines = page_lines(p, part, &mut Vec::new()).unwrap();
        assert_eq!(
            lines[3..],
            [
                "field Qty in Lines = Qty {styleexpr=Style}",
                "field Shown in Lines = var Shown: Text {}",
            ]
        );
        let fill = p.objects[part].routines.iter().position(|r| r.name == "Fill").unwrap();
        assert_eq!(
            routine_text(p, part, fill, &mut Vec::new()).unwrap(),
            "var DayKey: Text; begin foreach DayKey in Values.Keys() do begin Rec.\"Kind\" := option#Week; Rec.Insert(); end; \
             repeat Loaded := (Loaded and (Rec.\"Qty\" >= 2)); until (Rec.Next() = 0); end"
        );
        assert_eq!(
            table_lines(p, at("Render Line"), &mut Vec::new()).unwrap(),
            [
                "TableType = Temporary",
                "field 1 \"Kind\": Option Day,Week",
                "field 2 \"Day\": Date",
                "field 3 \"Qty\": Decimal",
                "key \"Kind\", \"Day\" {Clustered=true}"
            ]
        );
        let mut names = Names::default();
        assert!(names.line_is(
            "EnqueueBackgroundTask(Id, 97108, P)",
            "EnqueueBackgroundTask(Id, «K», P)"
        ));
        assert_eq!(names.bound('K'), Some("97108"));
    }
}
