//! The `CalcFields` and `LinkTableForceInsert` properties of a table element (xmlport/lt-y68-xmlport-shapes/
//! Y68XmlPortShapes.al, corpus master 0d01b0cc, BC 27.5, 28.5, 29.0).

use super::{Context, Layout, bool_prop};
use crate::members::FieldClass;

/// Lowers the two properties of table node `i` into `node`, or adds the reason the port is refused.
pub(super) fn lower(cx: &Context, layout: &Layout, i: usize, node: &mut al_ir::XmlNode, reasons: &mut Vec<String>) {
    let name = &layout.nodes[i].name;
    // CalcFields: on export the listed FlowFields are calculated on each row before its OnAfterGetRecord; the others read 0
    // there (Y68XmlPortShapes.al:567-621, real and temporary table elements). The values written are calculated with and
    // without the property. On import it changes nothing the insert triggers see (:623-640 Q4_CalcFields_Import).
    if let Some(v) = layout.property(i, "CalcFields") {
        if layout.format != al_ir::XmlFormat::Xml {
            reasons.push(format!(
                "table element {name}: CalcFields outside an Xml port (not pinned)"
            ));
            return;
        }
        let Some(t) = node.table else {
            reasons.push(format!("table element {name}: CalcFields without a table"));
            return;
        };
        for f in crate::scan::split_double(&v, ',') {
            let f = crate::scan::unquote_xmlport(&f);
            match cx.members[t]
                .fields
                .iter()
                .position(|x| x.name.eq_ignore_ascii_case(&f))
            {
                Some(p) if cx.members[t].fields[p].class == FieldClass::FlowField => node.calc_fields.push(p),
                _ => reasons.push(format!(
                    "table element {name}: CalcFields entry `{f}` is not a FlowField of its table (not pinned)"
                )),
            }
        }
    }
    // LinkTableForceInsert on a linked table element of an Xml port: import and export are the same with and without it
    // (Y68XmlPortShapes.al:715-775 Q6_LinkTableForceInsert_Import, _Export: the parent is inserted before its children
    // either way). Elsewhere it is not pinned.
    if let Some(v) = layout.property(i, "LinkTableForceInsert") {
        if bool_prop(&v).is_none()
            || layout.property(i, "LinkTable").is_none()
            || layout.format != al_ir::XmlFormat::Xml
        {
            reasons.push(format!(
                "table element {name}: LinkTableForceInsert outside a linked table element of an Xml port (not pinned)"
            ));
        }
    }
}
