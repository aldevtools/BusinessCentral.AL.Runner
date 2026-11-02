//! Container text elements (text elements with child elements) whose triggers run, and `currXMLport.Skip()` in them
//! (xmlport/lt-y60-xmlport-container-skip/Y60XpContainerSkip.al; the same shape as al-sema xmlports `container_tree_shape`).

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

/// A text element of only text attributes whose ancestors are all text elements (xmlport/lt-y67-xmlport-textattr-skip/
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
    /// xmlport/W7XmlPortTests.al:55 `Cont.AfterAssign=direct`). Below a table element it is not pinned.
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
        let text = elem.text();
        let text = if text.trim().is_empty() { String::new() } else { text };
        self.xp_assign_variable(inst, schema, node, text)
    }
}
