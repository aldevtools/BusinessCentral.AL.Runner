//! A VariableText port whose table element holds another table element (the nested shape).
//!
//! Export (xmlport/lt-df-xmlport-leads/DfXmlPortLeads.al:VariableText_Nested_Export, and the
//! staged probe xmlport/lt-y83-xmlport-shapes/Y83XpMiscTests.al:VariableText_Nested `export nested`): the first child record follows
//! the parent's values on the parent's line, the other child records follow on lines of their own, the table separator ends the
//! children of a parent, and the record separator joins the parents (so the last parent has no record separator after it).
//! A parent without a child record is not pinned.

use super::*;

/// The nested table element of `table`: its last child, when a table element follows the columns.
pub(super) fn child_table(schema: &Schema, table: usize) -> Option<usize> {
    let kids = schema.children(table);
    let last = *kids.last()?;
    (kids.len() > 1 && schema.node(last).kind == ir::XmlNodeKind::Table).then_some(last)
}

impl Session {
    /// The text of a nested export, before the encoding is applied.
    pub(super) fn xp_nested_text(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        table: usize,
        fields: &[usize],
        child: usize,
    ) -> Result<String, Signal> {
        let xp = schema.xp();
        let child_fields = self.xp_text_columns(schema, child, false)?;
        let mut parents: Vec<String> = Vec::new();
        let mut seps: Option<(String, String)> = None;
        for_each_row(self, inst, schema, table, |me, inst, schema| {
            let (fsep, fdelim) = (
                assigned::separator(inst, 1, xp.field_separator.as_deref(), ",")?,
                assigned::separator(inst, 0, xp.field_delimiter.as_deref(), "\"")?,
            );
            let rsep = assigned::separator(inst, 2, xp.record_separator.as_deref(), "\r\n")?;
            let tsep = assigned::separator(inst, 3, xp.table_separator.as_deref(), "\r\n\r\n")?;
            match &seps {
                Some((r, t)) if *r != rsep || *t != tsep => {
                    return unsupported("a separator set after the first row of a nested export (not pinned)");
                }
                _ => seps = Some((rsep.clone(), tsep.clone())),
            }
            let line = |me: &mut Session, inst: &Inst, schema: &Schema, cols: &[usize]| -> Result<String, Signal> {
                let mut cells = Vec::new();
                for &f in cols {
                    match me.xp_field_pass(inst, schema, f)? {
                        FieldPass::Value(v) => cells.push(format!("{fdelim}{v}{fdelim}")),
                        _ => {
                            return unsupported(
                                "a nested VariableText export that omits or stops a field (not pinned)",
                            );
                        }
                    }
                }
                Ok(cells.join(&fsep))
            };
            let head = line(me, inst, schema, fields)?;
            let mut children: Vec<String> = Vec::new();
            for_each_row(me, inst, schema, child, |me, inst, schema| {
                children.push(line(me, inst, schema, &child_fields)?);
                Ok(())
            })?;
            if children.is_empty() {
                return unsupported("a nested VariableText export of a record without a child record (not pinned)");
            }
            let mut text = format!("{head}{fsep}{}", children[0]);
            for c in &children[1..] {
                text.push_str(&rsep);
                text.push_str(c);
            }
            text.push_str(&tsep);
            parents.push(text);
            Ok(())
        })?;
        let rsep = match seps {
            Some((r, _)) => r,
            None => assigned::separator(inst, 2, xp.record_separator.as_deref(), "\r\n")?,
        };
        Ok(parents.join(&rsep))
    }

    /// The error an import of the nested shape ends with after a record of the parent table: the child table is expected and
    /// none is read (Y83XpMiscTests.al:VariableText_Nested `nested doc one` to `three`: the same error for every document, with
    /// nothing stored), or for an optional child table the empty line is an unexpected child (DfXmlPortLeads.al:
    /// VariableText_Nested_Import `header_then_lines`).
    pub(super) fn xp_nested_import_error(
        schema: &Schema,
        table: usize,
        child: usize,
        empty_line: bool,
    ) -> Option<Signal> {
        let c = schema.node(child);
        if c.min_zero {
            return empty_line.then(|| {
                xerr(
                    format!("The Element <{}> is unexpected.", c.xml_name),
                    "XmlPortUnexpectedElement",
                )
            });
        }
        Some(xerr(
            format!(
                "The Element <{}> is expected by Min Occurs value: Once. Element received: <{}>.",
                c.xml_name,
                schema.node(table).xml_name
            ),
            "XmlPortMetadata",
        ))
    }
}
