//! The columns of a VariableText or FixedText import line: how many values a record may hold, and the unbound text element
//! that takes the values after the others.

use super::*;

/// The unbound text element of an import line: the last column, a text element with `Unbound = true` on a VariableText port
/// takes every further value of the line, its OnAfterAssignVariable once per value, empty ones included, all in one record
/// (xmlport/lt-y60-xmlport-unbound-text/Y60XpUnboundText.al Unbound_Text_Import_ManyColumns, _ShortAndEmptyColumns,
/// _TrailingSeparator_QuotedValue; `MaxOccurs = Unbounded` beside it changes nothing: Unbound_Text_MaxOccursUnbounded).
/// An unbound element that is not the last column, or on a FixedText port, is not pinned.
pub(super) fn unbound_column(schema: &Schema, fields: &[usize], variable: bool) -> Result<Option<usize>, Signal> {
    let unbound: Vec<usize> = fields.iter().copied().filter(|&f| schema.node(f).unbound).collect();
    match unbound[..] {
        [] => Ok(None),
        [f] if variable && schema.node(f).kind == ir::XmlNodeKind::Text && fields.last() == Some(&f) => Ok(Some(f)),
        _ => unsupported("an unbound element of a text-format import that is not its last text column (not pinned)"),
    }
}

/// Whether a line of `values` values is pinned for the columns `fields`.
/// - As many values as columns.
/// - VariableText, a line that ends early: the columns it lacks are left as initialized when each is `MinOccurs = Zero`, a field
///   element (platform/lt-ag-contoso/AgXmlPortImport.al:678, xmlport 87567: `AUD;036;1;0.01` of 8 columns) or a text element
///   (Y60XpUnboundText.al Unbound_Text_Import_ShortAndEmptyColumns, line `a` of `col1`, `colx`).
/// - VariableText, values too many: one is left out (AgXmlPortImport.al:678 'too many columns'); when the last column is a text
///   element, any number is left out without an error (Y60XpUnboundText.al Unbound_Text_Import_WithoutUnbound_SameFile,
///   `a,b,c,d` into two text elements), and an unbound one takes them all (`unbound_column`).
pub(super) fn line_length_pinned(schema: &Schema, fields: &[usize], values: usize, variable: bool) -> bool {
    if values == fields.len() {
        return true;
    }
    if !variable {
        return false;
    }
    if values < fields.len() {
        return fields[values..].iter().all(|&f| {
            let n = schema.node(f);
            matches!(n.kind, ir::XmlNodeKind::Field | ir::XmlNodeKind::Text) && n.min_zero
        });
    }
    let last_text = fields
        .last()
        .is_some_and(|&f| schema.node(f).kind == ir::XmlNodeKind::Text);
    values == fields.len() + 1 || last_text
}
