//! A page field that shows rendered content (ADR-040 amendment 2026-11-12, P11): `TestField.Value()` of a control whose source
//! is a page variable of an unbounded Text and that shows the text unchanged gives the same text. Every other reader of the
//! control's text refuses (AssertEquals, the As* methods: A6, a test never fails with the mark in its message). Every other control refuses:
//! a format, an OptionCaption, `ExtendedDatatype` (RichContent is not measured), `MultiLine` and any other property that could
//! change what the control shows wait for probe (4) of the amendment's A9.

use super::*;

/// The properties a control may have and still show a text unchanged: they name, describe or place it.
const SHOWS_UNCHANGED: &[&str] = &["applicationarea", "caption", "tooltip"];

impl Session {
    /// The rendered content `v` holds, if it is a Text that holds some.
    fn rendered(v: &Value) -> Option<&str> {
        match v {
            Value::Text(t) | Value::Code(t) if t.has_marker() && al_builtins::rendered_text::holds(t.audited_str()) => {
                Some(t.audited_str())
            }
            _ => None,
        }
    }

    /// The text `TestField.Value()` of control `idx` gives for `v`: rendered content by the rule of this module, any other
    /// value as the control shows it (`control_text`).
    pub(super) fn field_value_text(&mut self, st: &St, idx: usize, v: &Value) -> Result<String, Signal> {
        match self.rendered_field_value(st, idx, v) {
            Some(shown) => shown,
            None => self.control_text_for(st, idx, v, super::auto_format_writer_cell::ControlRead::Value),
        }
    }

    /// What `TestField.Value()` of control `idx` gives for `v` when `v` holds rendered content; `None` for any other value.
    fn rendered_field_value(&self, st: &St, idx: usize, v: &Value) -> Option<Result<String, Signal>> {
        let t = Self::rendered(v)?;
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        let plain = matches!(v, Value::Text(_))
            && matches!(ctl.binding, ir::Binding::Var(_))
            && matches!(self.control_type(st, idx, v), Some(RtType::Text(None)))
            && ctl.props.iter().all(|(n, _)| SHOWS_UNCHANGED.contains(&n.as_str()));
        if !plain {
            return Some(Err(al_builtins::rendered_text::refusal(
                t,
                "a page control other than a plain field over a Text variable",
            )));
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ReportHtmlRenderedText);
        Some(Ok(t.to_string()))
    }

    /// The refusal of any other reader of a control's shown text (`control_text`: AssertEquals, the As* methods, a lookup, a
    /// request page's parameters) when `v` holds rendered content; `None` for any other value.
    pub(super) fn rendered_field_refusal(v: &Value) -> Option<Signal> {
        let t = Self::rendered(v)?;
        Some(al_builtins::rendered_text::refusal(
            t,
            "the text a page control shows, read by anything but TestField.Value()",
        ))
    }
}
