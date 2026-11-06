//! Text elements nested in each other below a table element, written as the plain tree (al-runner issue 7130; corpus PR 1158,
//! xmlports/shapes-properties/XpTableRootNestedText.al, BC 27.5, 28.5 and 29.0 alike).

use super::*;

/// A text element without triggers whose child nodes are leaf field elements, leaf text elements and table elements, or text elements
/// of the same kind (at least one child, no attributes, no triggers on the text elements): BC writes it once per row of the table
/// element above it, around its content (XpTableRootNestedText.al XNT_TxtField_Export, XNT_TxtTbl_Export, XNT_TxtTxtField_Export,
/// XNT_TxtTxtTbl_Export, XNT_NsTxtField_Export, XNT_NsTxtTbl_Export).
pub(super) fn plain_text_tree(schema: &Schema, c: usize) -> bool {
    let n = schema.node(c);
    let kids = schema.children(c);
    // A text element with its own value (`valued_leaf`) is pinned next to a table element, once, in the text element right below the
    // root table element (XNT_NsTblText_Export, XNT_NsDfltTblText_Export, XNT_TblText_*, XNT_TextTbl_*).
    let valued = kids.iter().filter(|&&k| valued_leaf(schema, k)).count();
    let valued_ok = valued == 0
        || (valued == 1
            && is_root_table(schema, n.parent)
            && kids.iter().any(|&k| schema.node(k).kind == ir::XmlNodeKind::Table));
    n.kind == ir::XmlNodeKind::Text
        && n.triggers.is_empty()
        && !kids.is_empty()
        && valued_ok
        && kids.iter().all(|&k| {
            let m = schema.node(k);
            match m.kind {
                ir::XmlNodeKind::Field => schema.children(k).is_empty(),
                ir::XmlNodeKind::Text if schema.children(k).is_empty() => {
                    m.triggers.is_empty() || valued_leaf(schema, k)
                }
                ir::XmlNodeKind::Text => plain_text_tree(schema, k),
                ir::XmlNodeKind::Table => true,
                _ => false,
            }
        })
}

/// A leaf text element whose only trigger is OnBeforePassVariable: the value it sets is written, once.
pub(super) fn valued_leaf(schema: &Schema, k: usize) -> bool {
    let m = schema.node(k);
    m.kind == ir::XmlNodeKind::Text
        && schema.children(k).is_empty()
        && !m.triggers.is_empty()
        && m.triggers.iter().all(|(name, _)| name == "onbeforepassvariable")
}

fn is_root_table(schema: &Schema, node: Option<usize>) -> bool {
    node.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table && schema.node(p).parent.is_none())
}

/// A `valued_leaf` in a pinned place: the only element of the root table element (XNT_PlainLeaf_Export, XNT_LeafRows_Export), or
/// in a `plain_text_tree` right below it. Its trigger's Skip, Break and Quit are not pinned there.
pub(super) fn plain_valued_leaf(schema: &Schema, k: usize) -> bool {
    let Some(p) = schema.node(k).parent else {
        return false;
    };
    valued_leaf(schema, k)
        && ((is_root_table(schema, Some(p)) && schema.children(p).len() == 1)
            || (schema.node(p).kind == ir::XmlNodeKind::Text
                && is_root_table(schema, schema.node(p).parent)
                && plain_text_tree(schema, p)))
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
        && (kids.iter().any(|&k| plain_text_tree(schema, k)) || matches!(kids[..], [only] if valued_leaf(schema, only)))
        && kids.iter().all(|&k| {
            let m = schema.node(k);
            (schema.children(k).is_empty() && m.triggers.is_empty() && !m.min_zero && m.kind == ir::XmlNodeKind::Field)
                || (m.kind == ir::XmlNodeKind::Text && schema.children(k).is_empty() && m.triggers.is_empty())
                || (kids.len() == 1 && valued_leaf(schema, k))
                || plain_text_tree(schema, k)
        })
}

/// A `plain_tree_root` whose import is pinned: MaxOccurs = Once and no field element of its own (XNT_TxtField_Import,
/// XNT_TxtTbl_Import, XNT_TxtTxtField_Import, XNT_TxtTxtTbl_Import, XNT_PlainLeaf_Import, XNT_TblText_Import, XNT_TextTbl_Import).
/// Other roots are exported only.
pub(super) fn plain_import_root(schema: &Schema, top: usize) -> bool {
    plain_tree_root(schema, top)
        && schema.node(top).max_unbounded == Some(false)
        && schema
            .children(top)
            .iter()
            .all(|&k| schema.node(k).kind != ir::XmlNodeKind::Field)
}

/// A `plain_text_tree` below the root table element `table` (its import is pinned there only; below another table element it is not).
pub(super) fn plain_import_tree(schema: &Schema, table: usize, c: usize) -> bool {
    is_root_table(schema, Some(table)) && plain_text_tree(schema, c)
}

/// A `plain_valued_leaf` right below the root table element `table`.
pub(super) fn plain_import_leaf(schema: &Schema, table: usize, c: usize) -> bool {
    is_root_table(schema, Some(table)) && plain_valued_leaf(schema, c)
}

impl Session {
    /// The import of a `plain_text_tree` below the root table element: its field elements assign the root's record (`assigned`), its
    /// table elements insert their rows, its leaf text elements are read as any text element; the root's row is saved by the caller
    /// after all of it (XpTableRootNestedText.al XNT_TxtField_Import, XNT_TxtTbl_Import, XNT_TxtTxtField_Import, XNT_TxtTxtTbl_Import,
    /// XNT_TblText_Import, XNT_TextTbl_Import: a root without fields of its own inserts a blank row 0). Triggers on the table
    /// elements and a LinkTable are not pinned there.
    pub(super) fn xp_import_plain_tree(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        elem: &Elem,
        assigned: &mut Vec<usize>,
    ) -> Result<(), Signal> {
        let mut seq = Sequence::new(schema, node);
        let mut seen: Vec<usize> = Vec::new();
        for ce in elem.elements() {
            let Some(child) = seq.accept(schema, ce)? else {
                continue;
            };
            let m = schema.node(child);
            match m.kind {
                ir::XmlNodeKind::Field => {
                    let pos = self.xp_assign_field_elem(inst, schema, child, ce)?;
                    assigned.push(pos);
                }
                ir::XmlNodeKind::Text if schema.children(child).is_empty() => {
                    self.xp_assign_leaf_text(inst, schema, child, ce)?;
                }
                ir::XmlNodeKind::Text => self.xp_import_plain_tree(inst, schema, child, ce, assigned)?,
                ir::XmlNodeKind::Table => {
                    if m.link_node.is_some() || !m.triggers.is_empty() {
                        return unsupported(
                            "XmlPort import of a table element with LinkTable or triggers inside a text element below the root table element (not pinned)",
                        );
                    }
                    let first = !seen.contains(&child);
                    seen.push(child);
                    self.xp_import_record(inst, schema, child, ce, first, false)?;
                }
                _ => return unsupported("XmlPort import of a text element with content inside a table element"),
            }
            if Self::xp_import_stopped(inst) {
                return Ok(());
            }
        }
        seq.finish(schema)
    }
}

impl Session {
    /// A `plain_valued_leaf`: its OnBeforePassVariable sets the text, written once where the schema has it (XpTableRootNestedText.al
    /// XNT_NsTblText_Export, XNT_TblText_Export, XNT_TextTbl_Export, XNT_PlainLeaf_Export). Skip, Break and Quit in that trigger
    /// are not pinned there.
    pub(super) fn xp_export_valued_leaf(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<Elem, Signal> {
        let v = match self.xp_pass_variable(inst, schema, node) {
            Err(Signal::Exit(None)) if Self::xp_quit(inst) => {
                return unsupported(
                    "Quit in OnBeforePassVariable of a text element with a value below a table element (not pinned)",
                );
            }
            other => other?,
        };
        if Self::xp_flow(inst).is_some() {
            return unsupported(
                "Skip or Break in OnBeforePassVariable of a text element with a value below a table element (not pinned)",
            );
        }
        let mut e = Elem::new(&doc_name(schema, node));
        e.children.push(Node::Text(v));
        Ok(e)
    }
}
