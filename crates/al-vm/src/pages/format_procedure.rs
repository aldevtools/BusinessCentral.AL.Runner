//! The AutoFormatExpression of a table field that calls a procedure, under a TestPage control that names no format of its own
//! (al-runner issue 7850). BC runs the procedure when the page fetches a row, never when a control is read
//! (pages/controls/AutoFormatProcedureControl.al:14-16; format-evaluate/QiTableFieldAutoFormat.al:449-455 for type 10).
//!
//! Pinned, for the shape of [`Session::format_procedure_open_pinned`] only (AutoFormatProcedureControl.al:223-257):
//! - The open runs it on a blank record of the table, then on the row (Y17AfProcedure.al:491 `hdr();hdr();hdr(H1);hdr(H1);` for
//!   types 1 and 2, QiTableFieldAutoFormat.al:449 `fmt();fmt(A);` for type 10): 2 runs at OpenView (AutoFormatProcedureControl.al:264).
//! - The GoToRecord after the open runs it once on the row (:265). After an OpenEdit only the sum of the open and the
//!   GoToRecord is measured (:269, 3), so an OpenEdit runs its fill only when its next statement is that GoToRecord and no AL
//!   code runs between them ([`Session::open_edit_then_go_to_record`]).
//! - `Value()`, `AsDecimal()`, `SetValue` and `AssertEquals` run nothing (:266-273; QiTableFieldAutoFormat.al:432-437 reads
//!   after the open); a read takes the precision of what the last run gave.
//! - A `Close()` in mode View runs nothing (:268). A `Close()` in mode Edit that saves a changed row runs it once (:274
//!   `calls=2`: the Close and the test's `Format(Amount, 0, 9)`, which runs it once, Y17AfProcedure.al:398 for types 1 and 2,
//!   Y43AfType11.al:343 for the format of a type 11 field at number 0).
//!
//! Any other page that would have to run such a procedure refuses at the fetch, and on a page whose open ran it every TestPage
//! step outside those refuses before it runs ([`Session::format_procedure_step`]).

use super::*;
use al_builtins::builtin_id::BuiltinId;
use super::state::FormatProcedureStep;
use al_builtins::page_op::{PageMethod, PageOp, PageRecv};
use al_ir::{Arg, Expr, Place, Stmt};

/// What a procedure control needs to run: (control index, the routine that evaluates the field's expression).
type ProcedureControl = (usize, (usize, usize));

impl Session {
    /// The controls of the page that name no AutoFormatExpression of their own over a table field whose AutoFormatExpression
    /// calls a procedure ([`calls_procedure`]).
    fn format_procedure_controls(&self, st: &St) -> Vec<usize> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        (0..page.controls.len())
            .filter(|&idx| page.controls[idx].prop("autoformatexpression").is_none())
            .filter(|&idx| {
                self.control_field(st, idx)
                    .and_then(|f| field_text(&f, "AutoFormatExpression"))
                    .is_some_and(|t| calls_procedure(&t))
            })
            .collect()
    }

    /// The page fetches its row (`populate_expressions`). A page without a procedure control runs nothing here. The open of a
    /// page in the pin runs the procedure on a blank record of the table, then on the row; the GoToRecord after it runs it on
    /// the row. Every other fetch of a page with such a control refuses: BC runs the procedure there too.
    pub(super) fn fill_format_procedures(&mut self, st: &St) -> Result<(), Signal> {
        let controls = self.format_procedure_controls(st);
        if controls.is_empty() {
            return Ok(());
        }
        st.borrow_mut().field_format_texts.clear();
        let (open, goto, step) = {
            let s = st.borrow();
            (s.open_read, s.format_procedure_goto, s.format_procedure_step)
        };
        if open {
            let control = match self.format_procedure_open_pinned(st, &controls)? {
                Ok(c) => c,
                Err(why) => return self.refuse_format_procedure_fetch(st, controls[0], why),
            };
            st.borrow_mut().format_procedure_step = FormatProcedureStep::None;
            let table = self.page_rec(st).borrow().table;
            let Value::Record(blank) = self.default_value(&RtType::Record(table, false)) else {
                return unsupported("an empty record of the page's source table");
            };
            self.run_format_procedure(st, control, blank)?;
            let row = self.page_rec(st);
            self.run_format_procedure(st, control, row)?;
            st.borrow_mut().format_procedure_step = FormatProcedureStep::Opened;
            return Ok(());
        }
        if goto && step == FormatProcedureStep::Opened && self.on_the_one_row(st)? {
            let control = self.format_procedure_control(st, &controls)?;
            let row = self.page_rec(st);
            self.run_format_procedure(st, control, row)?;
            st.borrow_mut().format_procedure_step = FormatProcedureStep::Landed;
            return Ok(());
        }
        self.refuse_format_procedure_fetch(st, controls[0], "fetch: only the open and the GoToRecord after it are")
    }

    fn refuse_format_procedure_fetch(&self, st: &St, idx: usize, why: &str) -> Result<(), Signal> {
        let field = self.control_field(st, idx).map(|f| f.name).unwrap_or_default();
        unsupported(format!(
            "a control over table field {field} whose AutoFormatExpression calls a procedure: BC runs it when the page fetches a \
             row, and this fetch is outside pages/controls/AutoFormatProcedureControl.al (not pinned: {why})"
        ))
    }

    /// The page stands on the one stored row of its view (the probe's tables hold one row, AutoFormatProcedureControl.al:188-198).
    fn on_the_one_row(&mut self, st: &St) -> Result<bool, Signal> {
        let on_row = {
            let s = st.borrow();
            s.cursor == Cursor::Row && !s.draft
        };
        Ok(on_row && self.view_row_count(st)? == 1)
    }

    /// The open of page `st` is the probe's (AutoFormatProcedureControl.al:97-145, :19-95, :223-257), checked clause by clause; the
    /// error names the first clause that does not hold. Ok: the one procedure control and the routine of its field's expression.
    fn format_procedure_open_pinned(
        &mut self,
        st: &St,
        controls: &[usize],
    ) -> Result<Result<ProcedureControl, &'static str>, Signal> {
        let opened = {
            let s = st.borrow();
            s.host.is_none()
                && !s.modal
                && s.opened_by_test
                && s.trapped_run.is_none()
                && matches!(s.mode, PageMode::View | PageMode::Edit)
        };
        if !opened {
            return Ok(Err("opened: only OpenView and OpenEdit of the test are"));
        }
        if let Err(why) = self.format_procedure_page_pinned(st) {
            return Ok(Err(why));
        }
        let [idx] = controls else {
            return Ok(Err("control: more than one control shows a field whose AutoFormatExpression calls a procedure"));
        };
        let control = match self.format_procedure_table_pinned(st, *idx) {
            Ok(c) => c,
            Err(why) => return Ok(Err(why)),
        };
        if !self.on_the_one_row(st)? {
            return Ok(Err("row: the view holds no row or more than one"));
        }
        if st.borrow().mode == PageMode::Edit && !self.open_edit_then_go_to_record() {
            return Ok(Err("GoToRecord: the statement after the OpenEdit is not a GoToRecord of the same TestPage"));
        }
        Ok(Ok(control))
    }

    /// The page and its controls are the probe's cards (AutoFormatProcedureControl.al:97-145): a Card that declares PageType,
    /// SourceTable, ApplicationArea = All and UsageCategory = None and nothing else, no trigger and no subscriber, page
    /// extensions without properties; one group without properties holding every field control, each bound to a field of Rec
    /// with ApplicationArea = All as its only property and no trigger.
    fn format_procedure_page_pinned(&self, st: &St) -> Result<(), &'static str> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let props = &engine.program.objects[object].properties;
        let declared = |name: &str, value: Option<&str>| {
            props.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case(name) && value.is_none_or(|want| unquote(v.trim()).eq_ignore_ascii_case(want))
            })
        };
        let page_ok = engine.object_facts[object].page_type == Some(al_ir::PageType::Card)
            && props.len() == 4
            && declared("PageType", Some("Card"))
            && declared("SourceTable", None)
            && declared("ApplicationArea", Some("All"))
            && declared("UsageCategory", Some("None"))
            && page.triggers.is_empty()
            && page.ext_triggers.is_empty()
            && page.extensions.iter().all(|&e| engine.program.objects[e].properties.is_empty())
            && !engine.events.any_subscriber_of(object);
        if !page_ok {
            return Err("page: the page declares a property, trigger, type or subscriber the probe's cards do not");
        }
        let groups: Vec<usize> = (0..page.controls.len())
            .filter(|&i| page.controls[i].kind == ir::ControlKind::Group)
            .collect();
        let [group] = groups[..] else {
            return Err("control: the page has no group or more than one");
        };
        let controls_ok = page.controls.iter().enumerate().all(|(i, c)| {
            let quiet = c.triggers.is_empty() && c.ext_triggers.is_empty();
            let shape = match c.kind {
                ir::ControlKind::Group => i == group && c.parent.is_none() && c.binding == ir::Binding::None && c.props.is_empty(),
                ir::ControlKind::Field => {
                    c.parent == Some(group)
                        && matches!(c.binding, ir::Binding::Field(_))
                        && matches!(&c.props[..], [(k, ir::PropVal::Lit(v))]
                            if k == "applicationarea" && unquote(v.trim()).eq_ignore_ascii_case("All"))
                }
                _ => false,
            };
            quiet && shape
        });
        if !controls_ok {
            return Err("control: a control is not a field of Rec in the one group, or declares a property or trigger");
        }
        Ok(())
    }

    /// The source table and the format field are the probe's (AutoFormatProcedureControl.al:42-95): the table declares
    /// DataClassification only, its extensions nothing; no trigger in the table or an extension, no subscriber; every field
    /// declares DataClassification only, but the field under control `idx`, a Decimal that adds AutoFormatType = 11 and the
    /// AutoFormatExpression `<global>.<procedure>(<global>.FieldNo(<field>))` (:53, :89), the one field of the table whose
    /// expression calls a procedure. `<global>` is a Record global (not temporary) of the object that declares the field, and the
    /// procedure is the probe's: it hands its parameter to a procedure of a codeunit local and returns what that gives (:34-39).
    fn format_procedure_table_pinned(&self, st: &St, idx: usize) -> Result<ProcedureControl, &'static str> {
        let engine = self.engine();
        let table = self.page_rec(st).borrow().table as usize;
        let objects = &engine.program.objects;
        let Some(t) = objects[table].table.as_ref() else {
            return Err("table: the source is no table");
        };
        let table_ok = matches!(&objects[table].properties[..], [(k, _)] if k.eq_ignore_ascii_case("DataClassification"))
            && t.extensions.iter().all(|&e| objects[e].properties.is_empty())
            && std::iter::once(table)
                .chain(t.extensions.iter().copied())
                .all(|o| objects[o].routines.iter().all(|r| !r.is_trigger))
            && !engine.events.any_subscriber_of(table);
        if !table_ok {
            return Err("table: the table declares a property, trigger or subscriber the probe's tables do not");
        }
        let ir::Binding::Field(pos) = page_of(&engine, st).controls[idx].binding else {
            return Err("control: the format control is bound to no field");
        };
        let only = |f: &ir::Field, names: &[&str]| {
            f.properties.len() == names.len()
                && names.iter().all(|n| f.properties.iter().any(|(k, _)| k.eq_ignore_ascii_case(n)))
        };
        // Fields numbered 2000000000 and up are the platform's system fields, which every table has, the probe's too.
        let fields_ok = t.fields.iter().enumerate().all(|(i, f)| {
            if f.id >= 2_000_000_000 {
                return true;
            }
            if i != pos {
                return only(f, &["DataClassification"]);
            }
            only(f, &["DataClassification", "AutoFormatType", "AutoFormatExpression"])
                && matches!(f.ty, RtType::Decimal)
                && field_text(f, "AutoFormatType").as_deref() == Some("11")
        });
        if !fields_ok {
            return Err("field: a field declares a type or property the probe's fields do not");
        }
        let f = &t.fields[pos];
        let (Some(text), Some(routine), None) =
            (field_text(f, "AutoFormatExpression"), f.auto_format_expression, f.auto_format_refusal.as_ref())
        else {
            return Err("expression: the AutoFormatExpression did not compile");
        };
        let Some((global, procedure)) = probe_expression(&text) else {
            return Err("expression: the AutoFormatExpression is not <global>.<procedure>(<global>.FieldNo(<field>))");
        };
        let receiver = objects[routine.0].globals.iter().find_map(|g| match g.ty {
            RtType::Record(r, false) if g.name.eq_ignore_ascii_case(global) => Some(r as usize),
            _ => None,
        });
        let Some(receiver) = receiver else {
            return Err("expression: the receiver is no Record global of the object that declares the field");
        };
        let mut named = objects[receiver]
            .routines
            .iter()
            .filter(|r| !r.is_trigger && r.name.eq_ignore_ascii_case(procedure));
        let (Some(r), None) = (named.next(), named.next()) else {
            return Err("procedure: the receiver's table has no one procedure of that name");
        };
        if !hands_on_to_a_codeunit(objects, r) {
            return Err("procedure: the procedure does more than hand its parameter to a codeunit's procedure");
        }
        Ok((idx, routine))
    }

    /// The statement that is calling `TestPage.OpenEdit` is a statement of its own, and the next one is the `GoToRecord` of the same
    /// TestPage variable on a plain variable, its answer unused: between the two no AL code runs, so what the OpenEdit alone ran
    /// cannot be read (AutoFormatProcedureControl.al:239-241 reads only after both).
    fn open_edit_then_go_to_record(&self) -> bool {
        let (Some(&object), Some(frame)) = (self.frames.last(), self.call_stack.last()) else {
            return false;
        };
        let engine = self.engine();
        let Some(r) = engine.program.objects.get(object).and_then(|o| o.routines.get(frame.routine)) else {
            return false;
        };
        open_then_go_to(r.body.stmts(), frame.pos)
    }

    /// Runs the format procedure of `control` once on `rec` and keeps what it gave. An error the procedure raises refuses: it is
    /// measured only for type 10 on the row (QiTableFieldAutoFormat.al:507-524).
    fn run_format_procedure(&mut self, st: &St, control: ProcedureControl, rec: Rec) -> Result<(), Signal> {
        let (idx, (object, routine)) = control;
        let value = match self.call_on_record(object, routine, rec, None, Vec::new()) {
            Ok((v, ..)) => v,
            Err(Signal::Error(_)) => {
                return unsupported(
                    "the AutoFormatExpression procedure of a control's field raised an error at a fetch (not pinned for type 11)",
                );
            }
            Err(e) => return Err(e),
        };
        let mut s = st.borrow_mut();
        s.field_format_texts.retain(|(i, _)| *i != idx);
        s.field_format_texts.push((idx, value));
        Ok(())
    }

    /// A TestPage step on page `st` before it runs. On a page whose open ran a format procedure only the probe's steps run
    /// (AutoFormatProcedureControl.al:223-257): reads of any control (`Value`, `AsDecimal`, `AssertEquals`); the first GoToRecord after
    /// the open; in mode Edit one SetValue of the format control after it; and `Close` after the GoToRecord in mode View, after the
    /// SetValue in mode Edit. Every other step refuses.
    pub(super) fn format_procedure_step(&self, st: &St, method: PageOp, target: Option<&ControlTarget>) -> Result<(), Signal> {
        use FormatProcedureStep as S;
        let (step, mode) = {
            let s = st.borrow();
            (s.format_procedure_step, s.mode)
        };
        if step == S::None {
            return Ok(());
        }
        let format_control = || match target {
            Some(ControlTarget::Control(i)) => self.format_procedure_controls(st) == [*i],
            _ => false,
        };
        let next = match (method.recv(), method.method()) {
            (PageRecv::TestField, PageMethod::Value | PageMethod::AsDecimal | PageMethod::AssertEquals) => Some(step),
            (PageRecv::TestPage, PageMethod::GoToRecord) if step == S::Opened => Some(step),
            (PageRecv::TestField, PageMethod::SetValue) if mode == PageMode::Edit && step == S::Landed && format_control() => {
                Some(S::Set)
            }
            (PageRecv::TestPage, PageMethod::Close)
                if (mode, step) == (PageMode::View, S::Landed) || (mode, step) == (PageMode::Edit, S::Set) =>
            {
                Some(step)
            }
            _ => None,
        };
        match next {
            Some(n) => {
                st.borrow_mut().format_procedure_step = n;
                Ok(())
            }
            None => unsupported(format!(
                "{method} on a page whose open ran an AutoFormatExpression procedure (not pinned: only the steps of \
                 pages/controls/AutoFormatProcedureControl.al:223-257 are)"
            )),
        }
    }

    /// `TestPage.GoToRecord`: [`Session::page_go_to_key_captions`], marking the fetch it makes. After an open that ran a format
    /// procedure the GoToRecord must fetch the row (`fill_format_procedures`): one that fetched nothing refuses.
    pub(super) fn page_go_to_record(&mut self, st: &St, key: &mut [Value]) -> Result<Value, Signal> {
        st.borrow_mut().format_procedure_goto = true;
        let r = self.page_go_to_key_captions(st, key);
        st.borrow_mut().format_procedure_goto = false;
        if r.is_ok() && st.borrow().format_procedure_step == FormatProcedureStep::Opened {
            return unsupported(
                "a GoToRecord that fetched no row on a page whose open ran an AutoFormatExpression procedure (not pinned)",
            );
        }
        r
    }

    /// Whether `TestPage.Close()` of page `st` saves the row a SetValue changed after an open that ran a format procedure: the
    /// procedure then runs once after it (`format_procedures_after_close`). A SetValue that changed no value is not pinned.
    pub(super) fn format_procedures_close_saves(&self, st: &St) -> Result<bool, Signal> {
        if st.borrow().format_procedure_step != FormatProcedureStep::Set {
            return Ok(false);
        }
        let changed = {
            let s = st.borrow();
            s.cursor == Cursor::Row && s.dirty && !s.draft
        } && !Self::same_values(&self.page_rec(st).borrow().fields, &self.page_xrec_fields(st));
        if !changed {
            return unsupported(
                "TestPage.Close after a SetValue that changes no value on a page whose open ran an AutoFormatExpression procedure \
                 (not pinned)",
            );
        }
        Ok(true)
    }

    /// After a `TestPage.Close()` of a page whose open ran a format procedure: the Close that saved a changed row
    /// (`format_procedures_close_saves`) runs it once on the row (AutoFormatProcedureControl.al:274); the page forgets the steps.
    pub(super) fn format_procedures_after_close(&mut self, st: &St, saved: bool) -> Result<(), Signal> {
        if st.borrow().format_procedure_step == FormatProcedureStep::None {
            return Ok(());
        }
        if saved {
            let controls = self.format_procedure_controls(st);
            let control = self.format_procedure_control(st, &controls)?;
            let row = self.page_rec(st);
            self.run_format_procedure(st, control, row)?;
        }
        let mut s = st.borrow_mut();
        s.field_format_texts.clear();
        s.format_procedure_step = FormatProcedureStep::None;
        Ok(())
    }

    /// The one procedure control of a page whose open ran it, with the routine of its field's expression.
    fn format_procedure_control(&self, st: &St, controls: &[usize]) -> Result<ProcedureControl, Signal> {
        match controls {
            [idx] => match self.control_field(st, *idx).and_then(|f| f.auto_format_expression) {
                Some(r) => Ok((*idx, r)),
                None => unsupported("the AutoFormatExpression of the format control did not compile"),
            },
            _ => unsupported("a page whose open ran an AutoFormatExpression procedure shows more than one such control"),
        }
    }

    /// What control `idx`'s format procedure gave at the last fetch that ran it, in the texts the probe measured: a blank text
    /// (AutoFormatProcedureControl.al:279), and `<precision, a:b><standard format, 0>`, which shows a places as
    /// `<Precision,a:b><Standard Format,0>` does (:260 `value-after` `7.00` with 2:2, :298 `value` `3` and `value-after` `8` with 0:0)
    /// and is handed on in that spelling. No fetch ran it: main's refusal; it gave another text: that text refuses.
    pub(super) fn format_procedure_text(&self, st: &St, idx: usize, field: &str) -> Result<String, Signal> {
        let s = st.borrow();
        let Some((_, value)) = s.field_format_texts.iter().find(|(i, _)| *i == idx) else {
            return unsupported(format!(
                "a control over table field {field} whose AutoFormatExpression calls a procedure (BC runs it when the row is fetched; not modelled)"
            ));
        };
        let text = match value {
            Value::Text(t) | Value::Code(t) => t.as_str(),
            _ => "",
        };
        let places = text
            .strip_prefix("<precision, ")
            .and_then(|r| r.strip_suffix("><standard format, 0>"))
            .filter(|p| p.split_once(':').is_some_and(|(a, b)| digits(a) && digits(b)));
        match (places, value) {
            (Some(p), _) => Ok(format!("<Precision,{p}><Standard Format,0>")),
            (None, Value::Text(t) | Value::Code(t)) if t.as_str().is_empty() => Ok(String::new()),
            _ => unsupported(format!(
                "the AutoFormatExpression procedure of table field {field} gave a format text no case measured: {}",
                value.describe()
            )),
        }
    }
}

/// An AutoFormatExpression as written calls a procedure: it is no Text literal and holds a call.
pub(super) fn calls_procedure(text: &str) -> bool {
    !is_text_literal(text) && text.contains('(')
}

/// `<global>.<procedure>(<global>.FieldNo(<field>))`, the same global twice, as the probe writes it
/// (AutoFormatProcedureControl.al:53, :89): (global, procedure).
fn probe_expression(text: &str) -> Option<(&str, &str)> {
    let (global, rest) = text.trim().split_once('.')?;
    let (procedure, args) = rest.split_once('(')?;
    let (again, field_no) = args.strip_suffix(')')?.split_once('.')?;
    let field = field_no
        .get(..8)
        .filter(|p| p.eq_ignore_ascii_case("FieldNo("))
        .and_then(|_| field_no[8..].strip_suffix(')'))?;
    let quoted = |s: &str| s.len() > 2 && s.starts_with('"') && s.ends_with('"') && !s[1..s.len() - 1].contains('"');
    (identifier(global) && identifier(procedure) && again.eq_ignore_ascii_case(global) && (identifier(field) || quoted(field)))
        .then_some((global, procedure))
}

fn identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Routine `r` is `exit(<codeunit local>.<procedure>(<parameter>, ...))` and nothing else (AutoFormatProcedureControl.al:34-39): it reads
/// and writes no variable of its table and no field of its record.
fn hands_on_to_a_codeunit(objects: &ir::Objects, r: &ir::Routine) -> bool {
    let strip = |mut e: &Expr| {
        while let Expr::Convert(_, inner) = e {
            e = inner;
        }
        e.clone()
    };
    let stmts: Vec<&Stmt> = r.body.stmts().iter().filter(|s| !matches!(s, Stmt::Pos(_))).collect();
    let [Stmt::Exit(Some(e))] = stmts[..] else {
        return false;
    };
    let Expr::Call {
        object,
        instance: Some(instance),
        args,
        try_call: false,
        ..
    } = strip(e)
    else {
        return false;
    };
    let declared_local = matches!(*instance, Expr::Load(Place::Local(n)) if n >= r.params);
    let parameters = args.iter().all(|a| match a {
        Arg::Value(e) => matches!(strip(e), Expr::Load(Place::Local(n)) if n < r.params),
        Arg::Ref(_) => false,
    });
    objects[object].kind == ir::ObjKind::Codeunit && declared_local && parameters
}

/// Somewhere in `stmts` (or a block inside them), position `pos` marks `<v>.OpenEdit()` as a statement, and the next statement is
/// `<v>.GoToRecord(<variable>)` with its answer unused.
fn open_then_go_to(stmts: &[Stmt], pos: u32) -> bool {
    let open = BuiltinId::must("TestPage.OpenEdit#1");
    let go_to = BuiltinId::must("TestPage.GoToRecord#1");
    let plain = |a: &Arg| {
        matches!(a, Arg::Ref(Place::Local(_) | Place::Global(_)) | Arg::Value(Expr::Load(Place::Local(_) | Place::Global(_))))
    };
    let here = stmts.windows(4).any(|w| match w {
        [
            Stmt::Pos(p),
            Stmt::Expr(Expr::Builtin { id: a, receiver: Some(ra), args: aa, .. }),
            Stmt::Pos(_),
            Stmt::Expr(Expr::Builtin { id: b, receiver: Some(rb), args: ba, ret_used: false }),
        ] => *p == pos && *a == open && aa.is_empty() && *b == go_to && ra == rb && matches!(&ba[..], [x] if plain(x)),
        _ => false,
    });
    here || stmts.iter().any(|s| match s {
        Stmt::If(_, a, b) => open_then_go_to(a, pos) || open_then_go_to(b, pos),
        Stmt::While(_, body) | Stmt::Repeat(body, _) | Stmt::AssertError(body) => open_then_go_to(body, pos),
        Stmt::For { body, .. } | Stmt::Foreach { body, .. } => open_then_go_to(body, pos),
        Stmt::Case { branches, otherwise, .. } => {
            branches.iter().any(|(_, b)| open_then_go_to(b, pos)) || open_then_go_to(otherwise, pos)
        }
        _ => false,
    })
}

/// `s` is one or more ASCII digits.
fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())
}

/// The value of property `name` of field `f`, trimmed.
fn field_text(f: &ir::Field, name: &str) -> Option<String> {
    f.properties
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.trim().to_string())
}
