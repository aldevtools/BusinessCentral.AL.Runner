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
    let valued_ok = valued == 0
        || (valued_place(schema, c)
            && (kids.iter().any(|&k| schema.node(k).kind == ir::XmlNodeKind::Table) || table_in_text_above(schema, c)));
    n.kind == ir::XmlNodeKind::Text
        && n.triggers.is_empty()
        && !kids.is_empty()
        && valued_ok
        && kids.iter().all(|&k| {
            let m = schema.node(k);
            match m.kind {
                ir::XmlNodeKind::Field => schema.children(k).is_empty(),
                ir::XmlNodeKind::Text if schema.children(k).is_empty() => {
                    no_export_trigger(m) || valued_leaf(schema, k)
                }
                ir::XmlNodeKind::Text => plain_text_tree(schema, k),
                ir::XmlNodeKind::Table => true,
                _ => false,
            }
        })
}

/// Whether a text element above text element `c`, below the nearest table element, holds a table element itself, and that nearest
/// table element is not the root (al-runner issue 7349; xmlports/shapes-properties/XpTextValueNestedTable.al, in the row of a table
/// element below the root text element: :1255 XTN_R1_Export, :1271 XTN_R2_Export, :1287 XTN_R3_Export, :1303 XTN_Around_Export,
/// :1319 XTN_TwoTexts_Export, :1335 XTN_TwoTables_Export, :1351 XTN_Mixed_Export, :1367 XTN_DeepTable_Export, :1383
/// XTN_DeepValue_Export, :1399 XTN_FieldValueAround_Export). BC writes the text element with content once and whole where the schema
/// places it. The same shapes below the root table element are not pinned and stay refused.
fn table_in_text_above(schema: &Schema, c: usize) -> bool {
    let mut at = schema.node(c).parent;
    let mut found = false;
    while let Some(p) = at {
        let n = schema.node(p);
        match n.kind {
            ir::XmlNodeKind::Table => return found && n.parent.is_some(),
            ir::XmlNodeKind::Text => {
                found |= schema
                    .children(p)
                    .iter()
                    .any(|&k| schema.node(k).kind == ir::XmlNodeKind::Table);
                at = n.parent;
            }
            _ => return false,
        }
    }
    false
}

/// A leaf text element whose only export trigger is OnBeforePassVariable: the value it sets is written, once. An
/// OnAfterAssignVariable next to it never runs on export and changes nothing it writes (al-runner issue 7355;
/// xmlports/shapes-properties/XpTextValueNestedTable.al:1439 XTT_BothValueTable_Export, :1463 XTT_BothTableValue_Export
/// `Pre;T1.BeforePass;Post;` with `<T1>x</T1>` for 0, 1 and 2 rows, before and after the table element).
pub(super) fn valued_leaf(schema: &Schema, k: usize) -> bool {
    let m = schema.node(k);
    m.kind == ir::XmlNodeKind::Text
        && schema.children(k).is_empty()
        && m.triggers.iter().any(|(name, _)| name == "onbeforepassvariable")
        && m.triggers
            .iter()
            .all(|(name, _)| name == "onbeforepassvariable" || name == IMPORT_ONLY)
}

/// The import trigger of a text element. On export it never runs: a leaf text element with it alone writes its variable as it
/// is (al-runner issue 7355; XpTextValueNestedTable.al:1427 XTT_AfterOnly_Export `<T1 />` with the log `Pre;Post;`, :1591
/// XTT_AfterOnlyPreset_Export `<T1>pre</T1>`).
const IMPORT_ONLY: &str = "onafterassignvariable";

/// Whether leaf text element `m` has no trigger an export runs (none, or OnAfterAssignVariable alone).
fn no_export_trigger(m: &ir::XmlNode) -> bool {
    m.triggers.iter().all(|(name, _)| name == IMPORT_ONLY)
}

/// Whether text element `c` or a text element below it has an OnAfterAssignVariable. The import of a plain tree with one is not
/// pinned (XpTextValueNestedTable.al pins the export of those ports only) and stays refused.
fn import_trigger_in(schema: &Schema, c: usize) -> bool {
    let m = schema.node(c);
    (m.kind == ir::XmlNodeKind::Text && m.triggers.iter().any(|(name, _)| name == IMPORT_ONLY))
        || schema.children(c).iter().any(|&k| import_trigger_in(schema, k))
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
        && ((is_root_table(schema, Some(p)) && (schema.children(p).len() == 1 || value_then_table_root(schema, p)))
            || (schema.node(p).kind == ir::XmlNodeKind::Text && valued_place(schema, p) && plain_text_tree(schema, p)))
}

/// A table element root of an Xml port without triggers, LinkTable or MinOccurs = Zero, whose nodes are leaf field elements, leaf
/// text elements without triggers and `plain_text_tree`s (at least one): written once per row, one root after the other, and once
/// with the record as it stands when there is none; MaxOccurs = Once writes the first row only (XpTableRootNestedText.al
/// XNT_TxtField_Export, XNT_TxtTxtTbl_Export, XNT_FieldTxt_Export, XNT_Rows_Export).
pub(super) fn plain_tree_root(schema: &Schema, top: usize) -> bool {
    schema.node(top).triggers.is_empty() && plain_tree_shape(schema, top)
}

/// A `plain_tree_root` as an export takes it: the root may also have triggers, and it may be a `value_then_table_root`. Its
/// OnPreXmlItem and OnAfterGetRecord run in document order around the triggers below it, the import triggers (OnAfterInitRecord,
/// OnBeforeInsertRecord, OnAfterInsertRecord, OnBeforeModifyRecord, OnAfterModifyRecord) never run (al-runner issue 7355;
/// xmlports/shapes-properties/XpTextValueNestedTable.al:1499 XTT_TableTriggers_Export
/// `Pre;orders.OnPreXmlItem;orders.OnAfterGetRecord;T1.BeforePass;r.OnPreXmlItem;r.OnAfterGetRecord;Post;`).
pub(super) fn plain_export_root(schema: &Schema, top: usize) -> bool {
    value_then_table_root(schema, top) || plain_tree_shape(schema, top)
}

/// The shape of a `plain_tree_root`, whatever triggers the root has. A `value_then_table_root` is not one: only its export is
/// pinned, so `plain_export_root` alone takes it and an import never sees it as a table root.
fn plain_tree_shape(schema: &Schema, top: usize) -> bool {
    let n = schema.node(top);
    let kids = schema.children(top);
    schema.xp().format == ir::XmlFormat::Xml
        && n.kind == ir::XmlNodeKind::Table
        && n.link_node.is_none()
        && !n.min_zero
        && (kids.iter().any(|&k| plain_text_tree(schema, k)) || matches!(kids[..], [only] if valued_leaf(schema, only)))
        && kids.iter().all(|&k| {
            let m = schema.node(k);
            (schema.children(k).is_empty() && m.triggers.is_empty() && !m.min_zero && m.kind == ir::XmlNodeKind::Field)
                || (m.kind == ir::XmlNodeKind::Text && schema.children(k).is_empty() && no_export_trigger(m))
                || (kids.len() == 1 && valued_leaf(schema, k))
                || plain_text_tree(schema, k)
        })
}

/// A root table element with MaxOccurs = Once and no triggers that holds, in this order, one leaf text element (a `valued_leaf`, or
/// one no export trigger of) and one table element without LinkTable or triggers whose nodes are leaf field elements without
/// triggers: BC writes the text element once, then the rows of the table element (al-runner issue 7355;
/// xmlports/shapes-properties/XpTextValueNestedTable.al:1535 XTT_RootTableValue_Export, :1559 XTT_RowDirect_Export
/// `<T1>x</T1>` with `Pre;T1.BeforePass;Post;`, :1571 XTT_RootTableAfterOnly_Export `<T1 />` with `Pre;Post;`). Pinned with one
/// row of the inner table element (`xp_export_table_pass` refuses another count); export only: `xp_import` refuses the shape.
pub(super) fn value_then_table_root(schema: &Schema, top: usize) -> bool {
    let n = schema.node(top);
    let [leaf, table] = schema.children(top)[..] else {
        return false;
    };
    let (l, t) = (schema.node(leaf), schema.node(table));
    schema.xp().format == ir::XmlFormat::Xml
        && n.kind == ir::XmlNodeKind::Table
        && n.parent.is_none()
        && n.triggers.is_empty()
        && n.link_node.is_none()
        && !n.min_zero
        && n.max_unbounded == Some(false)
        && l.kind == ir::XmlNodeKind::Text
        && schema.children(leaf).is_empty()
        && (no_export_trigger(l) || valued_leaf(schema, leaf))
        && t.kind == ir::XmlNodeKind::Table
        && t.triggers.is_empty()
        && t.link_node.is_none()
        && !t.min_zero
        && t.source_view.is_none()
        && !schema.children(table).is_empty()
        && schema.children(table).iter().all(|&f| {
            let m = schema.node(f);
            m.kind == ir::XmlNodeKind::Field && m.triggers.is_empty() && !m.min_zero && schema.children(f).is_empty()
        })
}

/// A `plain_tree_root` whose import is pinned: MaxOccurs = Once and no field element of its own (XNT_TxtField_Import,
/// XNT_TxtTbl_Import, XNT_TxtTxtField_Import, XNT_TxtTxtTbl_Import, XNT_PlainLeaf_Import, XNT_TblText_Import, XNT_TextTbl_Import),
/// no `valued_leaf` on the second text level or deeper and at most one in a text element (XpTextValueNestedTable.al pins those
/// shapes' export only). Other roots are exported only.
pub(super) fn plain_import_root(schema: &Schema, top: usize) -> bool {
    plain_tree_root(schema, top)
        && !import_trigger_in(schema, top)
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
    is_root_table(schema, Some(table)) && plain_text_tree(schema, c) && !import_trigger_in(schema, c)
}

/// A `plain_valued_leaf` right below the root table element `table`.
pub(super) fn plain_import_leaf(schema: &Schema, table: usize, c: usize) -> bool {
    is_root_table(schema, Some(table)) && plain_valued_leaf(schema, c) && !import_trigger_in(schema, c)
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
