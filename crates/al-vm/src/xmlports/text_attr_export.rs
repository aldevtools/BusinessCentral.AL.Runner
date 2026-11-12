//! Export of a text element with one field attribute and its own text below a temporary table element (al-runner issue 7865):
//! the shapes corpus PR 1366 measures (xmlports/shapes-properties/XpTextElemFieldAttrExport.al:468-533), under the assumption
//! `xmlport-text-elem-field-attr-export`. al-sema `xmlports/assumed_shapes.rs` `text_attr_export` decides the shape and marks
//! the element (`ir::XmlNode::text_attr_export`).

use super::*;
use al_builtins::assumptions::{Assumption, assume};

impl Session {
    /// The element in the measured shape: its OnBeforePassVariable, then the element, always written, with the attribute read
    /// after the trigger (:508) and the text when it is not empty (:468 `<MinValue equalsValid="true">5</MinValue>`, cleared
    /// `<MinValue equalsValid="true" />`; :478 the text set in OnAfterGetRecord).
    /// Skip, Break or Quit in the trigger refuses.
    pub(super) fn xp_export_text_attr(&mut self, inst: &Inst, schema: &Schema, child: usize) -> Result<Elem, Signal> {
        let text = self.xp_pass_variable(inst, schema, child)?;
        if Self::xp_flow(inst).is_some() {
            return unsupported(format!(
                "XmlPort export: Skip, Break or Quit in OnBeforePassVariable of the text element {} with a field attribute (not pinned)",
                schema.node(child).name
            ));
        }
        assume(Assumption::XmlportTextElemFieldAttrExport);
        let mut e = Elem::new(&doc_name(schema, child));
        self.xp_export_content(inst, schema, child, &mut e)?;
        if !text.is_empty() {
            e.children.push(Node::Text(text));
        }
        Ok(e)
    }
}
