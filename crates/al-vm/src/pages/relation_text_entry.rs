//! SetValue into a Text page variable with its own TableRelation (al-runner issue 8339). The reporter measured on BC 29.1 that
//! the control then holds the related row's stored key value: `CRONUS USA, INC.` typed into a page variable and a request page
//! variable with `TableRelation = Company` gives `CRONUS USA, Inc.`. The runner kept the typed text. Which value BC takes for
//! other values is not pinned either, so SetValue of any value but a blank text into such a control, bound to a variable of
//! the page or to a variable of the base page from an extension, is refused before anything runs. A control bound to a field
//! of Rec keeps its path (not measured by the reporter; this refusal does not cover it), and so does a Code variable: its value and the
//! stored key read the same in upper case.

use super::*;

impl Session {
    pub(super) fn refuse_text_entry_with_relation(&self, st: &St, idx: usize, value: &Value) -> Result<(), Signal> {
        let blank = matches!(value, Value::Text(t) | Value::Code(t) if t.trim().is_empty());
        let variable_with_relation = {
            let engine = self.engine();
            let c = &page_of(&engine, st).controls[idx];
            matches!(c.binding, ir::Binding::Var(_) | ir::Binding::BaseVar(_)) && c.prop("tablerelation").is_some()
        };
        if !blank && variable_with_relation && matches!(self.control_type(st, idx, value), Some(RtType::Text(_))) {
            return unsupported(
                "SetValue of a text into a Text page variable with a TableRelation (whether BC takes the related row's stored value is not pinned)",
            );
        }
        Ok(())
    }
}
