//! SetValue of a text into a Text page variable with its own TableRelation (al-runner issue 8339). The reporter measured on BC
//! 29.1 that the control then holds the related row's stored key value: `CRONUS USA, INC.` typed into a page variable and a
//! request page variable with `TableRelation = Company` gives `CRONUS USA, Inc.`. The runner kept the typed text. Which value
//! BC takes for other typed texts is not pinned either, so SetValue of a text that is not blank into such a control is refused
//! before anything runs. A control bound to a field of Rec keeps its path (pages/controls/P115Lookups.al:108), and so does a
//! Code variable: its value and the stored key read the same in upper case.

use super::*;

impl Session {
    pub(super) fn refuse_text_entry_with_relation(&self, st: &St, idx: usize, value: &Value) -> Result<(), Signal> {
        let typed = match value {
            Value::Text(t) | Value::Code(t) => !t.trim().is_empty(),
            _ => false,
        };
        let variable_with_relation = {
            let engine = self.engine();
            let c = &page_of(&engine, st).controls[idx];
            matches!(c.binding, ir::Binding::Var(_)) && c.prop("tablerelation").is_some()
        };
        if typed && variable_with_relation && matches!(self.control_type(st, idx, value), Some(RtType::Text(_))) {
            return unsupported(
                "SetValue of a text into a Text page variable with a TableRelation (whether BC takes the related row's stored value is not pinned)",
            );
        }
        Ok(())
    }
}
