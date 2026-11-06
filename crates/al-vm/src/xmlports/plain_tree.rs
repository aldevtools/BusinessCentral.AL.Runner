//! Text elements nested in each other below a table element, written as the plain tree (al-runner issue 7130; corpus branch
//! al-runtime-rs-staging-r7130, xmlports/shapes-properties/XpTableRootNestedText.al, BC 27.5, 28.5 and 29.0 alike).

use super::*;

/// A text element without triggers whose child nodes are leaf field elements, leaf text elements and table elements, or text elements
/// of the same kind (at least one child, no attributes, no triggers on the text elements): BC writes it once per row of the table
/// element above it, around its content (XpTableRootNestedText.al XNT_TxtField_Export, XNT_TxtTbl_Export, XNT_TxtTxtField_Export,
/// XNT_TxtTxtTbl_Export, XNT_NsTxtField_Export, XNT_NsTxtTbl_Export).
pub(super) fn plain_text_tree(schema: &Schema, c: usize) -> bool {
    let n = schema.node(c);
    let kids = schema.children(c);
    n.kind == ir::XmlNodeKind::Text
        && n.triggers.is_empty()
        && !kids.is_empty()
        && kids.iter().all(|&k| {
            let m = schema.node(k);
            match m.kind {
                ir::XmlNodeKind::Field => schema.children(k).is_empty(),
                ir::XmlNodeKind::Text if schema.children(k).is_empty() => m.triggers.is_empty(),
                ir::XmlNodeKind::Text => plain_text_tree(schema, k),
                ir::XmlNodeKind::Table => true,
                _ => false,
            }
        })
}

/// A table element root of an Xml port without triggers, LinkTable or MinOccurs = Zero, whose nodes are leaf field elements, leaf
/// text elements without triggers and `plain_text_tree`s (at least one): written once per row, one root after the other, and once
/// with the record as it stands when there is none; MaxOccurs = Once writes the first row only (XpTableRootNestedText.al
/// XNT_TxtField_Export, XNT_TxtTxtTbl_Export, XNT_FieldTxt_Export, XNT_Rows_Export).
pub(super) fn plain_tree_root(schema: &Schema, top: usize) -> bool {
    let n = schema.node(top);
    let kids = schema.children(top);
    schema.xp().format == ir::XmlFormat::Xml
        && n.kind == ir::XmlNodeKind::Table
        && n.triggers.is_empty()
        && n.link_node.is_none()
        && !n.min_zero
        && kids.iter().any(|&k| plain_text_tree(schema, k))
        && kids.iter().all(|&k| {
            let m = schema.node(k);
            (schema.children(k).is_empty() && m.triggers.is_empty() && !m.min_zero && m.kind == ir::XmlNodeKind::Field)
                || (m.kind == ir::XmlNodeKind::Text && schema.children(k).is_empty() && m.triggers.is_empty())
                || plain_text_tree(schema, k)
        })
}
