//! The text of an object as a rule pinned for ONE outside reporter's repro compares it: the page's properties, its controls
//! with their properties and triggers, its own triggers, and for a table its fields, keys and code, each line written in a
//! plain AL-like form (`pinned_body::page_lines`, `table_lines`, `routine_text`). The pin states the repro's lines and requires
//! them all, in order, with equality (al-runner PR 7838 review, round 2: an entry condition that checks only names or a
//! subset also runs on pages the repro does not show). Anything the renderer does not know gives `None`: the object is not
//! the pinned one.

use al_builtins::value::RtType;
use al_ir as ir;

/// The calls a rendered body makes to procedures (object, routine): the pin compares their bodies too (`routine_text`).
pub(super) type Callees = Vec<(usize, usize)>;

/// The lines of page `object`: its properties as written (`PageType = Worksheet`), then each control
/// (`field Quantity in Rows = Quantity {applicationarea=All}` and a line per trigger), then each page trigger
/// (`trigger OnOpenPage: begin ... end`). `None` for a page with an extension, an extension trigger, a refusal, a control
/// declared by an extension, or anything else the renderer does not write.
pub(super) fn page_lines(program: &ir::Program, object: usize, callees: &mut Callees) -> Option<Vec<String>> {
    let o = &program.objects[object];
    let page = o.page.as_ref()?;
    if !page.extensions.is_empty() || !page.ext_triggers.is_empty() || !page.unsupported.is_empty() {
        return None;
    }
    let mut lines: Vec<String> = o.properties.iter().map(|(k, v)| format!("{k} = {v}")).collect();
    let table = o.rec_slot.and_then(|s| match o.globals.get(s)?.ty {
        RtType::Record(t, _) => program.objects.get(t as usize)?.table.as_ref(),
        _ => None,
    });
    for c in &page.controls {
        let plain = c.owner.is_none()
            && c.ext_triggers.is_empty()
            && c.run_object.is_none()
            && c.action_ref.is_none()
            && c.prop_texts.is_empty()
            && c.var_field.is_none()
            && c.var_slot.is_none()
            && c.expr_source.is_none()
            && !c.expr_call
            && c.array_element.is_none()
            && !c.bare_quoted_ext_source
            && c.source_text.is_none()
            && !c.factbox;
        if !plain {
            return None;
        }
        let kind = match c.kind {
            ir::ControlKind::Field => "field",
            ir::ControlKind::Group => "group",
            ir::ControlKind::Repeater => "repeater",
            ir::ControlKind::Action => "action",
            ir::ControlKind::ActionGroup => "actiongroup",
            ir::ControlKind::Part => "part",
            _ => return None,
        };
        let parent = match c.parent {
            None => "-".to_string(),
            Some(p) => page.controls.get(p)?.name.clone(),
        };
        let binding = match c.binding {
            ir::Binding::None => String::new(),
            ir::Binding::Field(f) => format!(" = {}", table?.fields.get(f)?.name),
            _ => return None,
        };
        let mut props = Vec::new();
        for (k, v) in &c.props {
            let ir::PropVal::Lit(v) = v else { return None };
            props.push(format!("{k}={v}"));
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
/// "Line No."`) and each routine (`trigger OnModify: ...`, `procedure X: ...`). `None` for a table with an extension or a
/// refusal, or a field with an initial value or a relation.
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
        if f.init.is_some()
            || f.relation_table.is_some()
            || !f.relation_tables.is_empty()
            || f.auto_increment
            || !f.stored
        {
            return None;
        }
        lines.push(format!("field {} \"{}\": {}", f.id, f.name, type_text(program, &f.ty)?));
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
        lines.push(format!(
            "{kind} {}: {}",
            routine.name,
            routine_text(program, object, r, callees)?
        ));
    }
    Some(lines)
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

/// Whether page `object` reads, line for line, as `want` (`page_lines`), and nothing subscribes to its events.
pub(super) fn page_is(engine: &crate::Engine, object: usize, want: &[&str], callees: &mut Callees) -> bool {
    !engine.events.any_subscriber_of(object)
        && page_lines(&engine.program, object, callees)
            .is_some_and(|have| have.iter().map(String::as_str).eq(want.iter().copied()))
}

/// Whether table `object` reads, line for line, as `want` (`table_lines`), and nothing subscribes to its events.
pub(super) fn table_is(engine: &crate::Engine, object: usize, want: &[&str], callees: &mut Callees) -> bool {
    !engine.events.any_subscriber_of(object)
        && table_lines(&engine.program, object, callees)
            .is_some_and(|have| have.iter().map(String::as_str).eq(want.iter().copied()))
}

/// Whether every procedure in `callees` reads as one of `want`, each written `"<object>".<procedure>: <body>`, calls
/// nothing itself, and its object has no subscriber.
pub(super) fn callees_are(engine: &crate::Engine, callees: &Callees, want: &[&str]) -> bool {
    callees.iter().all(|&(o, r)| {
        let mut inner = Vec::new();
        let Some(body) = routine_text(&engine.program, o, r, &mut inner) else {
            return false;
        };
        let text = format!(
            "\"{}\".{}: {body}",
            engine.program.objects[o].name, engine.program.objects[o].routines[r].name
        );
        inner.is_empty() && !engine.events.any_subscriber_of(o) && want.contains(&text.as_str())
    })
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
        RtType::Record(t, false) => format!("Record \"{}\"", program.objects.get(*t as usize)?.name),
        RtType::Codeunit(c) => format!("Codeunit \"{}\"", program.objects.get(*c as usize)?.name),
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
                _ => return None,
            },
            ir::Expr::Load(p) => self.place(p)?,
            // A conversion the compiler inserts to the type a parameter or an operator wants.
            ir::Expr::Convert(_, inner) => self.expr(inner, callees)?,
            ir::Expr::Unary(ir::UnOp::Not, x) => format!("(not {})", self.expr(x, callees)?),
            ir::Expr::Binary(op, a, b) => {
                let op = match op {
                    ir::BinOp::Add => "+",
                    ir::BinOp::Sub => "-",
                    ir::BinOp::Mul => "*",
                    ir::BinOp::IntDiv => "div",
                    ir::BinOp::Eq => "=",
                    ir::BinOp::Ne => "<>",
                    _ => return None,
                };
                format!("({} {op} {})", self.expr(a, callees)?, self.expr(b, callees)?)
            }
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
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
