//! Container text elements (text elements with child elements) whose triggers run, and `currXMLport.Skip()` in them
//! (xmlports/shapes-properties/Y60XpContainerSkip.al; the same shape as al-sema xmlports `container_tree_shape`).

use super::*;

/// A text element with child elements whose nodes are only text attributes, leaf text elements, text elements of only text
/// attributes, and text elements of this same shape.
pub(super) fn container_tree(schema: &Schema, c: usize) -> bool {
    let kids = schema.children(c);
    schema.node(c).kind == ir::XmlNodeKind::Text
        && kids.iter().any(|&k| schema.node(k).kind == ir::XmlNodeKind::Text)
        && kids.iter().all(|&k| match schema.node(k).kind {
            ir::XmlNodeKind::TextAttr => true,
            ir::XmlNodeKind::Text => {
                schema.children(k).is_empty() || attrs_only(schema, k) || container_tree(schema, k)
            }
            _ => false,
        })
}

/// A text element of only text attributes whose ancestors are all text elements (xmlports/shapes-properties/
/// Y67XpTextAttrSkip.al:298-374; the same shape as al-sema xmlports `attributed_outside_table`). The export writes it as a text
/// element right below a table element (`text_in_table`): its OnBeforePassVariable, then its attributes'.
pub(super) fn attributed_outside_table(schema: &Schema, c: usize) -> bool {
    let mut up = schema.node(c).parent;
    while let Some(p) = up {
        if schema.node(p).kind != ir::XmlNodeKind::Text {
            return false;
        }
        up = schema.node(p).parent;
    }
    schema.node(c).kind == ir::XmlNodeKind::Text && !schema.children(c).is_empty() && attrs_only(schema, c)
}

/// A `container_tree` that holds a trigger the export has to run: on itself or on a container below it.
pub(super) fn triggered_container(schema: &Schema, c: usize) -> bool {
    fn has(schema: &Schema, c: usize) -> bool {
        !schema.node(c).triggers.is_empty()
            || schema
                .children(c)
                .into_iter()
                .any(|k| schema.node(k).kind == ir::XmlNodeKind::Text && container_tree(schema, k) && has(schema, k))
    }
    container_tree(schema, c) && has(schema, c)
}

/// A `container_tree` right below a table element whose triggers are only on its leaf text elements: no trigger on itself, on a
/// container below it, on a text element of only text attributes or on a text attribute. The TaxCategory of a TaxSubtotal and of an
/// AllowanceCharge in Microsoft's PEPPOL 3.0 ports (`ID`, `Percent`, `TaxExemptionReason` that skips when empty, `TaxScheme > ID`).
/// Pinned by xmlports/shapes-properties/R6924eTreeInTable.al:287-288, :296-297, :305-306 (PEPPOL Tests, codeunits 139235, 139236).
pub(super) fn leaf_triggered_tree(schema: &Schema, c: usize, table: usize) -> bool {
    fn quiet(schema: &Schema, c: usize) -> bool {
        schema.node(c).triggers.is_empty()
            && schema.children(c).into_iter().all(|k| {
                let n = schema.node(k);
                match n.kind {
                    ir::XmlNodeKind::TextAttr => n.triggers.is_empty(),
                    ir::XmlNodeKind::Text if schema.children(k).is_empty() => true,
                    ir::XmlNodeKind::Text => quiet(schema, k),
                    _ => false,
                }
            })
    }
    schema.node(table).kind == ir::XmlNodeKind::Table
        && schema.node(c).parent == Some(table)
        && container_tree(schema, c)
        && quiet(schema, c)
}

impl Session {
    /// The export of a `triggered_container`: its OnBeforePassVariable first; `currXMLport.Skip()` there leaves out the element
    /// with all below it and their triggers do not run (Y60XpContainerSkip.al Container_Skip_BelowRoot_Export `period;after;post;`,
    /// Container_Nested_BothTriggers_Export 'inv only' `billing;tail;post;`). Else its attributes and elements in schema order; when
    /// all of them are left out it is written empty (`<cac:Billing />` in 'bill only', `<cac:Attachment />` in
    /// Container_TwoLevelsBelowTable_RowSkip_Export 'blob empty'). Its own text, and Break in its trigger, are not pinned.
    pub(super) fn xp_export_triggered_container(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
    ) -> Result<Option<Elem>, Signal> {
        let own = self.xp_pass_variable(inst, schema, node)?;
        match Self::xp_flow(inst) {
            Some(Flow::Skip) => {
                Self::xp_clear_flow(inst);
                return Ok(None);
            }
            Some(Flow::Quit) | None => {}
            Some(_) => return unsupported("Break in OnBeforePassVariable of a container text element (not pinned)"),
        }
        if !own.is_empty() {
            return unsupported("XmlPort export of a container text element with its own text (not pinned)");
        }
        self.xp_export_container(inst, schema, node).map(Some)
    }

    /// On import a container's OnAfterAssignVariable runs after its children, with its own text (empty without one:
    /// Y60XpContainerSkip.al Container_NestedTriggers_Import `refid=x;invdoc=[];billing=[];`, `<Billing/>` once `billing=[];`;
    /// xmlports/W7XmlPortTests.al:55 `Cont.AfterAssign=direct`). Below a table element it is not pinned.
    pub(super) fn xp_container_assigned(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        elem: &Elem,
    ) -> Result<(), Signal> {
        if schema.node(node).trigger("onafterassignvariable").is_none() {
            return Ok(());
        }
        let mut up = schema.node(node).parent;
        while let Some(p) = up {
            if schema.node(p).kind == ir::XmlNodeKind::Table {
                return unsupported(
                    "XmlPort import of a container text element with OnAfterAssignVariable below a table element (not pinned)",
                );
            }
            up = schema.node(p).parent;
        }
        // An empty element (`<Endpoint SchemeId="X" />`) leaves the variable as it was and the trigger runs once
        // (Y83XpMiscTests.al:1088 ContainerTriggers_AttributeOnly 'import empty endpoint': `ep-aav(0088);` after OnPreXmlPort set it).
        if elem.self_closed {
            return self.xp_run_trigger(inst, schema, node, "onafterassignvariable");
        }
        let text = elem.text();
        let text = if text.trim().is_empty() { String::new() } else { text };
        self.xp_assign_variable(inst, schema, node, text)
    }
}

/// The root text element with triggers beside table elements (xmlports/shapes-properties/Y83XpMiscTests.al ContainerTriggers_Nested;
/// the same test as al-sema xmlports `root_table_shape`): table elements without LinkTable (at least one) and text elements that are
/// leaves, hold only text attributes or are container trees.
pub(super) fn root_with_table(schema: &Schema, top: usize) -> bool {
    let kids = schema.children(top);
    let table = |c: usize| schema.node(c).kind == ir::XmlNodeKind::Table && schema.node(c).link_node.is_none();
    schema.node(top).kind == ir::XmlNodeKind::Text
        && kids.iter().any(|&c| table(c))
        && kids.iter().all(|&c| {
            table(c)
                || (schema.node(c).kind == ir::XmlNodeKind::Text
                    && (schema.children(c).is_empty() || attrs_only(schema, c) || container_tree(schema, c)))
        })
}

impl Session {
    /// A root with triggers runs them only in the shape above, on an Xml port without InlineSchema and with a text element root
    /// (the rest of the formats is not pinned).
    pub(super) fn xp_root_triggers_pinned(schema: &Schema, top: usize) -> Result<bool, Signal> {
        let n = schema.node(top);
        if n.kind != ir::XmlNodeKind::Text || n.triggers.is_empty() {
            return Ok(false);
        }
        let xp = schema.xp();
        if xp.format == ir::XmlFormat::Xml && !xp.inline_schema && root_with_table(schema, top) {
            return Ok(true);
        }
        unsupported(
            "XmlPort run of a root text element with triggers outside an Xml port with table elements (not pinned)",
        )
    }

    /// The root's OnBeforePassVariable, before its attributes and children (ContainerTriggers_Nested 'export': `root-bpv;rw-agr(1);`).
    /// Skip or Break in it, and its own text, are not pinned.
    pub(super) fn xp_export_root_trigger(&mut self, inst: &Inst, schema: &Schema, top: usize) -> Result<(), Signal> {
        let own = self.xp_pass_variable(inst, schema, top)?;
        if Self::xp_flow(inst).is_some() {
            return unsupported("Skip, Break or Quit in OnBeforePassVariable of the root text element (not pinned)");
        }
        if !own.is_empty() {
            return unsupported("XmlPort export of a root text element with its own text (not pinned)");
        }
        Ok(())
    }
}

impl Session {
    /// A `leaf_triggered_tree`: its own text refuses, then it is written as any container; Break in a leaf's
    /// OnBeforePassVariable refuses (see the arm in `xp_export_content`).
    pub(super) fn xp_export_leaf_triggered_tree(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        child: usize,
    ) -> Result<Elem, Signal> {
        if !self.xp_pass_variable(inst, schema, child)?.is_empty() {
            return unsupported(
                "XmlPort export of a container text element below a table element with its own text (not pinned)",
            );
        }
        let e = self.xp_export_container(inst, schema, child)?;
        if matches!(Self::xp_flow(inst), Some(Flow::Break | Flow::Unbound)) {
            return unsupported(
                "Break in OnBeforePassVariable of a leaf inside a container text element below a table element (not pinned)",
            );
        }
        Ok(e)
    }
}
