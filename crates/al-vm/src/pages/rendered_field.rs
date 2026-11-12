//! A page field that shows rendered content (ADR-040 amendment 2026-11-12, P11): `TestField.Value()` of a control whose source
//! is a page variable of an unbounded Text and that shows the text unchanged gives the same text. Every other control refuses:
//! a format, an OptionCaption, `ExtendedDatatype` (RichContent is not measured), `MultiLine` and any other property that could
//! change what the control shows wait for probe (4) of the amendment's A9.

use super::*;

/// The properties a control may have and still show a text unchanged: they name, describe or place it.
const SHOWS_UNCHANGED: &[&str] = &["applicationarea", "caption", "tooltip"];

impl Session {
    /// The text control `idx` shows for `v` when `v` holds rendered content; `None` for any other value.
    pub(super) fn rendered_field_text(&self, st: &St, idx: usize, v: &Value) -> Option<Result<String, Signal>> {
        let Value::Text(t) = v else {
            return None;
        };
        if !t.has_marker() || !al_builtins::rendered_text::holds(t.as_str()) {
            return None;
        }
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        let plain = matches!(ctl.binding, ir::Binding::Var(_))
            && matches!(self.control_type(st, idx, v), Some(RtType::Text(None)))
            && ctl.props.iter().all(|(n, _)| SHOWS_UNCHANGED.contains(&n.as_str()));
        if !plain {
            return Some(Err(al_builtins::rendered_text::refusal(
                t.as_str(),
                "a page control other than a plain field over a Text variable",
            )));
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ReportHtmlRenderedText);
        Some(Ok(t.as_str().to_string()))
    }
}
