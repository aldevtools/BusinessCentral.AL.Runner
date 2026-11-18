//! SetValue into a Text page variable with its own TableRelation (al-runner issue 8339). The reporter measured on BC 29.1 that
//! the control then holds the related row's stored key value: `CRONUS USA, INC.` typed into a page variable and a request page
//! variable with `TableRelation = Company` gives `CRONUS USA, Inc.`. The runner kept the typed text. Which value BC takes for
//! other values is not pinned either, so SetValue of any value but a blank text into such a control, bound to a variable of
//! the page or to a variable of the base page from an extension, is refused before anything runs. A control bound to a field
//! of Rec keeps its path (not measured by the reporter; this refusal does not cover it), and so does a Code variable: its value and the
//! stored key read the same in upper case. So does a relation, without a condition, to one table of the workspace with
//! `TableType = Temporary` (`relates_to_temporary_table`): such a table stores no row a value could be taken from.

use super::relation_lookup::{simple_relation, tables_named};
use super::*;

impl Session {
    pub(super) fn refuse_text_entry_with_relation(&self, st: &St, idx: usize, value: &Value) -> Result<(), Signal> {
        let blank = matches!(value, Value::Text(t) | Value::Code(t) if t.trim().is_empty());
        let (variable_with_relation, temporary_target) = {
            let engine = self.engine();
            let c = &page_of(&engine, st).controls[idx];
            let relation = match c.prop("tablerelation") {
                Some(ir::PropVal::Lit(t)) => Some(relates_to_temporary_table(&engine, t)),
                Some(_) => Some(false),
                None => None,
            };
            (
                matches!(c.binding, ir::Binding::Var(_) | ir::Binding::BaseVar(_)) && relation.is_some(),
                relation == Some(true),
            )
        };
        if !blank
            && variable_with_relation
            && !temporary_target
            && matches!(self.control_type(st, idx, value), Some(RtType::Text(_)))
        {
            return unsupported(
                "SetValue of a text into a Text page variable with a TableRelation (whether BC takes the related row's stored value is not pinned)",
            );
        }
        Ok(())
    }
}

/// Whether a control's own TableRelation `text` is `Table` or `Table.Field`, with or without a `where(..)`, and names
/// exactly one table of the workspace, whose `TableType` is `Temporary`. A relation with an `if` condition does not.
fn relates_to_temporary_table(engine: &Engine, text: &str) -> bool {
    let Some((table, _)) = simple_relation(before_where(text)) else {
        return false;
    };
    matches!(tables_named(engine, &table).as_slice(), [t] if engine.object_facts[*t].temporary())
}

/// `text` up to its first `where` word outside double quotes (all of it when it has none).
fn before_where(text: &str) -> &str {
    let mut quoted = false;
    for (i, c) in text.char_indices() {
        if c == '"' {
            quoted = !quoted;
        } else if !quoted && text.get(i..i + 5).is_some_and(|w| w.eq_ignore_ascii_case("where")) {
            let starts_word = !text[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_');
            let ends_word = !text[i + 5..].starts_with(|c: char| c.is_alphanumeric() || c == '_');
            if starts_word && ends_word {
                return &text[..i];
            }
        }
    }
    text
}
