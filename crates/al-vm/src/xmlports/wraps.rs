//! Text elements with content inside a table element, beyond the shapes of `shape.rs` (xmlport/lt-y60-xmlport-text-in-table/
//! Y60XpTextInTable.al; the same shapes as al-sema xmlports `containers`).

use super::*;

/// A text element whose child nodes are only table elements without LinkTable (at least one): `Wrap > Mid`, `Sub > Child`.
pub(super) fn table_wrap(schema: &Schema, c: usize) -> bool {
    let kids = schema.children(c);
    schema.node(c).kind == ir::XmlNodeKind::Text
        && !kids.is_empty()
        && kids.iter().all(|&k| {
            let n = schema.node(k);
            n.kind == ir::XmlNodeKind::Table && n.link_node.is_none()
        })
}

/// A text element right below table element `table` whose child nodes are field elements of that table (with only attributes
/// below them, at least one) and `table_wrap` text elements: `Info > Name (unit), Note, Sub > Child`.
pub(super) fn field_wrap(schema: &Schema, c: usize, table: usize) -> bool {
    let n = schema.node(c);
    let kids = schema.children(c);
    n.kind == ir::XmlNodeKind::Text
        && n.parent == Some(table)
        && schema.node(table).kind == ir::XmlNodeKind::Table
        && kids.iter().any(|&k| schema.node(k).kind == ir::XmlNodeKind::Field)
        && kids.iter().all(|&k| {
            let m = schema.node(k);
            match m.kind {
                ir::XmlNodeKind::Field => {
                    m.table_node == Some(table)
                        && schema.children(k).iter().all(|&a| {
                            matches!(schema.node(a).kind, ir::XmlNodeKind::TextAttr | ir::XmlNodeKind::FieldAttr)
                        })
                }
                ir::XmlNodeKind::Text => table_wrap(schema, k),
                _ => false,
            }
        })
}

/// A text element right below a table element with text attributes, leaf text and field elements (at least one) and text
/// elements of only text attributes (at least one): `Ticket kind > Code, Title lang`.
pub(super) fn attributed_leaves(schema: &Schema, c: usize) -> bool {
    let n = schema.node(c);
    let kids = schema.children(c);
    let leaf = |k: usize| {
        matches!(schema.node(k).kind, ir::XmlNodeKind::Text | ir::XmlNodeKind::Field) && schema.children(k).is_empty()
    };
    let attributed = |k: usize| schema.node(k).kind == ir::XmlNodeKind::Text && attrs_only(schema, k);
    n.kind == ir::XmlNodeKind::Text
        && n.parent.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table)
        && kids.iter().any(|&k| leaf(k))
        && kids.iter().any(|&k| attributed(k))
        && kids
            .iter()
            .all(|&k| schema.node(k).kind == ir::XmlNodeKind::TextAttr || leaf(k) || attributed(k))
}

impl Session {
    /// Export of a `table_wrap` right below a table element or below a `field_wrap`, or of a `field_wrap`: its OnBeforePassVariable,
    /// then its nodes in schema order. A table element below it without LinkTable lists all its rows under every row above, and
    /// one without rows leaves the text element empty (Y60XpTextInTable.al TextWrap_Table_Nested_Export, TextWrap_Q3_NoRows
    /// `<Wrap />`, TextWrap_Info_Fields_Sub_Export `row H1;a;b;info;unit;sub;child 1;child 2;`). Skip there and its own text are not
    /// pinned.
    pub(super) fn xp_export_wrap(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<Elem, Signal> {
        let own = self.xp_pass_variable(inst, schema, node)?;
        match Self::xp_flow(inst) {
            Some(Flow::Quit) | None => {}
            Some(_) => return unsupported("Skip or Break in OnBeforePassVariable of a text element that wraps a table element (not pinned)"),
        }
        if !own.is_empty() {
            return unsupported("XmlPort export of a text element that wraps a table element, with its own text (not pinned)");
        }
        self.xp_export_container(inst, schema, node)
    }

    /// Import of an `attributed_leaves` element: its attributes, then its elements in document order (an attributed one with its
    /// attributes before its text), then its own OnAfterAssignVariable with its own text (Y60XpTextInTable.al
    /// TextWrap_Attr_Leaves_Import `kind=k;code=c;lang=en;title=t;ticket=[];ins C;`; a missing attribute of the inner element:
    /// `The required attribute 'lang' is missing.`). Elements inside the inner element are not pinned.
    pub(super) fn xp_import_attributed_leaves(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        elem: &Elem,
        assigned: &mut Vec<usize>,
    ) -> Result<(), Signal> {
        self.xp_import_attrs(inst, schema, node, elem, Some(assigned))?;
        let mut seq = Sequence::new(schema, node);
        for ce in elem.elements() {
            let Some(child) = seq.accept(schema, ce)? else {
                continue;
            };
            match schema.node(child).kind {
                ir::XmlNodeKind::Field => assigned.push(self.xp_assign_field(inst, schema, child, &ce.text())?),
                _ if schema.children(child).is_empty() => self.xp_assign_leaf_text(inst, schema, child, ce)?,
                _ => {
                    if ce.elements().next().is_some() {
                        return unsupported(
                            "XmlPort import of elements inside a text element of text attributes inside a text element below a table element (not pinned)",
                        );
                    }
                    self.xp_import_attrs(inst, schema, child, ce, None)?;
                    self.xp_assign_variable(inst, schema, child, ce.text())?;
                }
            }
        }
        seq.finish(schema)?;
        let text = elem.text();
        let text = if text.trim().is_empty() { String::new() } else { text };
        self.xp_assign_variable(inst, schema, node, text)
    }
}
