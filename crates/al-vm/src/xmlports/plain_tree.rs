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
    // Text elements with their own value (`valued_leaf`) are pinned next to a table element, each once, in a `valued_place`.
    let valued = kids.iter().filter(|&&k| valued_leaf(schema, k)).count();
    let valued_ok =
        valued == 0 || (valued_place(schema, c) && kids.iter().any(|&k| schema.node(k).kind == ir::XmlNodeKind::Table));
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

/// Where a text element `c` may hold `valued_leaf`s next to a table element (written in schema order, each value once): below a
/// table element through text elements only, at any depth, in the root table element or in the row of another table element:
/// - right below the root table element (XpTableRootNestedText.al XNT_NsTblText_Export, XNT_NsDfltTblText_Export, XNT_TblText_*,
///   XNT_TextTbl_*);
/// - two text levels below it (al-runner issue 7263; xmlports/shapes-properties/XpTextValueNestedTable.al:954
///   XTV_RootTextsTableThenValue_Export, :969 XTV_RootTextsValueThenTable_Export, :984 XTV_RootTextsNamespaces_Export);
/// - three and four text levels below it, and two value elements in one text element (al-runner issue 7309; the same file:
///   :1060 XTV_Root3TextsTableThenValue_Export, :1073
///   XTV_Root3TextsValueThenTable_Export, :1089 XTV_Root3TextsNamespaces_Export, :1109 XTV_Root4TextsTableThenValue_Export, :1122
///   XTV_Root3TextsTwoValues_Export, :1135 XTV_Root2TextsTwoValues_Export);
/// - one to three text levels inside the row of a table element that is not the root, with one to three value elements before,
///   around or after the table element (:999 XTV_RowTextValueThenTable_Export, :1029 XTV_RowTextTableThenValue_Export, :1044
///   XTV_RowTextFieldValueThenTable_Export, :1148 XTV_RowTextTwoValuesThenTable_Export, :1164
///   XTV_RowTextValueTableValue_Export, :1177 XTV_RowTextTableThenTwoValues_Export, :1190 XTV_RowTextThreeValuesThenTable_Export,
///   :1216 XTV_RowDeepTwoValuesThenTable_Export, :1229 XTV_RowDeep3OneValueThenTable_Export, :1242
///   XTV_RowFieldTwoValuesThenTable_Export); one row of the outer table element.
fn valued_place(schema: &Schema, c: usize) -> bool {
    let mut at = schema.node(c).parent;
    while let Some(p) = at {
        match schema.node(p).kind {
            ir::XmlNodeKind::Table => return true,
            ir::XmlNodeKind::Text => at = schema.node(p).parent,
            _ => return false,
        }
    }
    false
}

/// A `valued_leaf` in a pinned place: the only element of the root table element (XNT_PlainLeaf_Export, XNT_LeafRows_Export), or
/// in a `plain_text_tree` in a `valued_place`. Its trigger's Skip, Break and Quit are not pinned there.
pub(super) fn plain_valued_leaf(schema: &Schema, k: usize) -> bool {
    let Some(p) = schema.node(k).parent else {
        return false;
    };
    valued_leaf(schema, k)
        && ((is_root_table(schema, Some(p)) && schema.children(p).len() == 1)
            || (schema.node(p).kind == ir::XmlNodeKind::Text && valued_place(schema, p) && plain_text_tree(schema, p)))
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
/// XNT_TxtTbl_Import, XNT_TxtTxtField_Import, XNT_TxtTxtTbl_Import, XNT_PlainLeaf_Import, XNT_TblText_Import, XNT_TextTbl_Import),
/// no `valued_leaf` on the second text level or deeper and at most one in a text element (XpTextValueNestedTable.al pins those
/// shapes' export only). Other roots are exported only.
pub(super) fn plain_import_root(schema: &Schema, top: usize) -> bool {
    plain_tree_root(schema, top)
        && schema.node(top).max_unbounded == Some(false)
        && schema.children(top).iter().all(|&k| {
            schema.node(k).kind != ir::XmlNodeKind::Field
                && !value_below_first_text_level(schema, k)
                && schema.children(k).iter().filter(|&&g| valued_leaf(schema, g)).count() <= 1
        })
}

/// Whether a text element below text element `c` holds a `valued_leaf`.
fn value_below_first_text_level(schema: &Schema, c: usize) -> bool {
    schema.children(c).iter().any(|&k| {
        schema.node(k).kind == ir::XmlNodeKind::Text
            && (schema.children(k).iter().any(|&g| valued_leaf(schema, g)) || value_below_first_text_level(schema, k))
    })
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
