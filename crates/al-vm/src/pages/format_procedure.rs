//! The AutoFormatExpression of a table field that calls a procedure, under a TestPage control that names no format of its own
//! (al-runner issue 7850). BC runs the procedure when the page fetches the row, never when the control is read
//! (pages/controls/AutoFormatProcedureControl.al:14-16): 2 times at OpenView and at OpenEdit, 1 at a GoToRecord, 1 at the Close that saves a changed row (`close-edit` `calls=2` counts the test's own `Format` of the
//! stored field too, :215), none at a Close without a change, `Value()`, `AsDecimal()`, `SetValue` or `AssertEquals`. A read
//! takes the precision of what the last fetch gave (`3.25` and `7.00` with 2:2 and with a blank text, `3` and `8` with 0:0).
//!
//! Every other fill and every other page keeps main's answer: the procedure does not run there and a read of the control
//! refuses (`Session::format_procedure_text` answers `None`).

use super::*;

/// The page properties the probe's cards declare (AutoFormatProcedureControl.al "PG6 AF Card", "PG6 AF Ext Card").
const PROBE_PAGE_PROPERTIES: [&str; 4] = ["pagetype", "sourcetable", "applicationarea", "usagecategory"];

impl Session {
    /// The controls of the page whose format comes from a procedure of their table field's AutoFormatExpression: (control
    /// index, the routine that evaluates it).
    fn format_procedure_controls(&self, st: &St) -> Vec<(usize, (usize, usize))> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        (0..page.controls.len())
            .filter(|&idx| page.controls[idx].prop("autoformatexpression").is_none())
            .filter_map(|idx| {
                let f = self.control_field(st, idx)?;
                let text = field_text(&f, "AutoFormatExpression")?;
                (text.contains('(') && f.auto_format_refusal.is_none())
                    .then_some(())
                    .and(f.auto_format_expression)
                    .map(|r| (idx, r))
            })
            .collect()
    }

    /// The page and its controls are the probe's (AutoFormatProcedureControl.al:42-145): a Card, not a part and not run
    /// modally, opened with OpenView or OpenEdit; no page trigger, no page event subscriber, no property but the probe's;
    /// controls are groups and fields bound to Rec without triggers, a page extension's too; the source table, its table
    /// extensions and its fields declare no trigger, the table declares no property but DataClassification (:44), and nothing subscribes to the table. Each control with a format procedure
    /// declares no format property of its own, and its field has AutoFormatType 11 and an AutoFormatExpression
    /// `<variable>.<procedure>(...)` whose variable is a Record global of the object that declares the field.
    fn format_procedure_pinned(&self, st: &St, controls: &[(usize, (usize, usize))]) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let shape = {
            let s = st.borrow();
            s.host.is_none() && !s.modal && matches!(s.mode, PageMode::View | PageMode::Edit)
        } && engine.object_facts[object].page_type == Some(al_ir::PageType::Card)
            && page.triggers.is_empty()
            && page.ext_triggers.is_empty()
            && !engine.events.any_subscriber_of(object)
            && engine.program.objects[object]
                .properties
                .iter()
                .all(|(k, _)| PROBE_PAGE_PROPERTIES.contains(&k.to_ascii_lowercase().as_str()));
        let control_kinds = page.controls.iter().all(|c| {
            let bound = match c.kind {
                ir::ControlKind::Group => c.binding == ir::Binding::None,
                ir::ControlKind::Field => matches!(c.binding, ir::Binding::Field(_)),
                _ => false,
            };
            bound && c.triggers.is_empty() && c.ext_triggers.is_empty()
        });
        let own_format = |idx: usize| {
            ["autoformattype", "autoformatexpression", "decimalplaces"]
                .iter()
                .any(|n| page.controls[idx].prop(n).is_some())
        };
        let fields = controls.iter().all(|&(idx, (declared, _))| {
            let Some(f) = self.control_field(st, idx) else {
                return false;
            };
            let receiver = field_text(&f, "AutoFormatExpression")
                .and_then(|t| t.split_once('.').map(|(r, _)| r.trim().to_string()));
            let global_record = receiver.is_some_and(|r| {
                engine.program.objects[declared]
                    .globals
                    .iter()
                    .any(|g| g.name.eq_ignore_ascii_case(&r) && matches!(g.ty, RtType::Record(..)))
            });
            !own_format(idx) && field_text(&f, "AutoFormatType").as_deref() == Some("11") && global_record
        });
        let table = self.page_rec(st).borrow().table as usize;
        let table_quiet = std::iter::once(table)
            .chain(
                engine.program.objects[table]
                    .table
                    .as_ref()
                    .map(|t| t.extensions.clone())
                    .unwrap_or_default(),
            )
            .all(|o| engine.program.objects[o].routines.iter().all(|r| !r.is_trigger))
            && !engine.events.any_subscriber_of(table)
            && engine.program.objects[table]
                .properties
                .iter()
                .all(|(k, _)| k.eq_ignore_ascii_case("DataClassification"));
        shape && control_kinds && fields && table_quiet
    }

    /// A row fill of the page (`populate_expressions`) on the one stored row of its view (the probe's tables hold one row,
    /// AutoFormatProcedureControl.al:188-198): the open's fill runs each format procedure 2 times, the fill of a GoToRecord
    /// (`format_procedure_goto`) 1 time; each control keeps what its last run gave. Any other fill, and any fill of a page
    /// outside the pin, runs nothing and forgets what the controls kept.
    pub(super) fn fill_format_procedures(&mut self, st: &St) -> Result<(), Signal> {
        let controls = self.format_procedure_controls(st);
        if controls.is_empty() {
            return Ok(());
        }
        st.borrow_mut().field_format_texts.clear();
        let on_row = {
            let s = st.borrow();
            s.cursor == Cursor::Row && !s.draft
        };
        let runs = match (st.borrow().open_read, st.borrow().format_procedure_goto) {
            (true, false) => 2,
            (false, true) => 1,
            _ => 0,
        };
        if runs == 0 || !on_row || !self.format_procedure_pinned(st, &controls) || self.view_row_count(st)? != 1 {
            return Ok(());
        }
        for _ in 0..runs {
            self.run_format_procedures(st, &controls)?;
        }
        Ok(())
    }

    /// Runs each format procedure once on the page's Rec and keeps the text it gave.
    fn run_format_procedures(&mut self, st: &St, controls: &[(usize, (usize, usize))]) -> Result<(), Signal> {
        for &(idx, (object, routine)) in controls {
            let rec = self.page_rec(st);
            let text = match self.call_on_record(object, routine, rec, None, Vec::new())?.0 {
                Value::Text(t) | Value::Code(t) => t.into_string(),
                other => {
                    return unsupported(format!(
                        "the AutoFormatExpression procedure of a control's field gave a {}",
                        other.describe()
                    ));
                }
            };
            let mut s = st.borrow_mut();
            s.field_format_texts.retain(|(i, _)| *i != idx);
            s.field_format_texts.push((idx, text));
        }
        Ok(())
    }

    /// `TestPage.GoToRecord`: [`Session::page_go_to_key_captions`], marking the fill it makes.
    pub(super) fn page_go_to_record(&mut self, st: &St, key: &mut [Value]) -> Result<Value, Signal> {
        st.borrow_mut().format_procedure_goto = true;
        let r = self.page_go_to_key_captions(st, key);
        st.borrow_mut().format_procedure_goto = false;
        r
    }

    /// Whether `TestPage.Close()` of `st` saves a changed row of a page in the pin, before the close runs: the procedures then run
    /// once after it (`format_procedures_after_close`).
    pub(super) fn format_procedures_close_saves(&self, st: &St) -> bool {
        let changed = {
            let s = st.borrow();
            s.mode == PageMode::Edit
                && s.cursor == Cursor::Row
                && s.dirty
                && !s.draft
                && !s.field_format_texts.is_empty()
        } && !Self::same_values(&self.page_rec(st).borrow().fields, &self.page_xrec_fields(st));
        if !changed {
            return false;
        }
        let controls = self.format_procedure_controls(st);
        !controls.is_empty() && self.format_procedure_pinned(st, &controls)
    }

    /// The Close that saved a changed row (`format_procedures_close_saves`) runs each format procedure once
    /// (AutoFormatProcedureControl.al:260 `close-edit`).
    pub(super) fn format_procedures_after_close(&mut self, st: &St) -> Result<(), Signal> {
        let controls = self.format_procedure_controls(st);
        self.run_format_procedures(st, &controls)?;
        st.borrow_mut().field_format_texts.clear();
        Ok(())
    }

    /// What control `idx`'s format procedure gave at the last fill that ran it, in the texts the probe measured: a blank text
    /// (AutoFormatProcedureControl.al:279), and `<precision, a:b><standard format, 0>`, which shows a places as
    /// `<Precision,a:b><Standard Format,0>` does (:260 `value-after` `7.00` with 2:2, :298 `value` `3` and `value-after` `8` with
    /// 0:0) and is handed on in that spelling. `None` when no pinned fill ran it or it gave another text: the read refuses.
    pub(super) fn format_procedure_text(&self, st: &St, idx: usize) -> Option<String> {
        let s = st.borrow();
        let (_, text) = s.field_format_texts.iter().find(|(i, _)| *i == idx)?;
        let places = text
            .strip_prefix("<precision, ")
            .and_then(|r| r.strip_suffix("><standard format, 0>"))
            .filter(|p| p.split_once(':').is_some_and(|(a, b)| digits(a) && digits(b)));
        match places {
            Some(p) => Some(format!("<Precision,{p}><Standard Format,0>")),
            None => text.is_empty().then(String::new),
        }
    }
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
