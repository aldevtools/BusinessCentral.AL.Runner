//! Export of a text element with one field attribute and its own text below a temporary table element (al-runner issue 7865):
//! the shapes corpus PR 1366 measures (xmlports/shapes-properties/XpTextElemFieldAttrExport.al:468-533), under the assumption
//! `xmlport-text-elem-field-attr-export`. al-sema lets the element's OnBeforePassVariable through for the same shape
//! (`containers::field_attr_text_shape`).

use super::*;
use al_builtins::assumptions::{Assumption, assume};

/// `child`, a text element right below the table element `table`, in the measured shape. Each fact by equality:
/// - an export port (Direction = Export, Format = Xml; every port of XpTextElemFieldAttrExport.al, :46-47);
/// - `table` temporary (UseTemporary) with MinOccurs = Zero, right below the root text element (:57-60);
/// - `child` with MaxOccurs = Once and no NamespacePrefix (:64; MinOccurs Zero at :65, not set at :204), no trigger (:111) or
///   only OnBeforePassVariable (:68);
/// - one field attribute below it, of a field of `table`, without Occurrence and without a trigger (:66).
pub(super) fn field_attr_text(schema: &Schema, child: usize, table: usize) -> Option<usize> {
    let xp = schema.xp();
    let t = schema.node(table);
    let c = schema.node(child);
    let below_root = t.parent.is_some_and(|p| {
        let p = schema.node(p);
        p.parent.is_none() && p.kind == ir::XmlNodeKind::Text
    });
    let [attr] = schema.children(child)[..] else {
        return None;
    };
    let a = schema.node(attr);
    let triggers_pinned = match &c.triggers[..] {
        [] => true,
        [(name, _)] => name == "onbeforepassvariable",
        _ => false,
    };
    (xp.direction == ir::XmlDirection::Export
        && xp.format == ir::XmlFormat::Xml
        && t.kind == ir::XmlNodeKind::Table
        && t.use_temporary
        && t.min_zero
        && below_root
        && c.kind == ir::XmlNodeKind::Text
        && c.max_unbounded == Some(false)
        && c.ns_prefix.is_none()
        && triggers_pinned
        && a.kind == ir::XmlNodeKind::FieldAttr
        && a.table_node == Some(table)
        && a.required.is_none()
        && a.triggers.is_empty())
    .then_some(attr)
}

impl Session {
    /// Whether the field attribute `attr` is of a type the probe measures: Boolean (XpTextElemFieldAttrExport.al:33) or Text
    /// (:34, the case at :518).
    pub(super) fn xp_text_attr_type_measured(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        attr: usize,
    ) -> Result<bool, Signal> {
        let a = schema.node(attr);
        let (Some(table), Some(pos)) = (a.table_node, a.field) else {
            return Ok(false);
        };
        let rec = self.xp_record(inst, schema.node(table))?;
        let t = self.table(&rec)?;
        Ok(matches!(t.fields[pos].ty, RtType::Boolean | RtType::Text(_)))
    }

    /// The element in the measured shape: its OnBeforePassVariable, then the element, always written, with the attribute read
    /// after the trigger (:508) and the text when it is not empty (:468 `<MinValue equalsValid="true">5</MinValue>`, cleared
    /// `<MinValue equalsValid="true" />`; :478 the text set in OnAfterGetRecord). An attribute of a type the probe does not
    /// measure, and Skip, Break or Quit in the trigger, refuse.
    pub(super) fn xp_export_text_attr(&mut self, inst: &Inst, schema: &Schema, child: usize) -> Result<Elem, Signal> {
        let [attr] = schema.children(child)[..] else {
            unreachable!("field_attr_text holds one attribute");
        };
        if !self.xp_text_attr_type_measured(inst, schema, attr)? {
            return unsupported(format!(
                "XmlPort export of the text element {} with OnBeforePassVariable and a field attribute of this type (not pinned)",
                schema.node(child).name
            ));
        }
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
