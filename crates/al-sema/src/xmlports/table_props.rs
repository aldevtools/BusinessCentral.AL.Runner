//! The `CalcFields` and `LinkTableForceInsert` properties of a table element (xmlports/shapes-properties/
//! Y68XmlPortShapes.al, corpus master 0d01b0cc, BC 27.5, 28.5, 29.0).

use super::{Context, Layout, bool_prop};
use crate::members::FieldClass;

/// Lowers the two properties of table node `i` into `node`, or adds the reason the port is refused (`import_reasons`: only its
/// import).
pub(super) fn lower(
    cx: &Context,
    layout: &Layout,
    i: usize,
    node: &mut al_ir::XmlNode,
    reasons: &mut Vec<String>,
    import_reasons: &mut Vec<String>,
) {
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
    force_insert(layout, i, reasons, import_reasons);
}

/// `LinkTableForceInsert` on table node `i`. An export writes the same with and without it, on a linked and an unlinked table
/// element of an Xml port, true or false, and on a VariableText port (Y68XmlPortShapes.al:761 Q6_LinkTableForceInsert_Export;
/// XpLinkTableForceInsertUnlinked.al:734 ForceOnUnlinked_Export, :751 ForceOnUnlinked_Export_OtherPorts, :768
/// ForceOnTableElement_VariableText `text export`). An import is the same as without it for `true` on a linked table element
/// (Y68XmlPortShapes.al:717 Q6_LinkTableForceInsert_Import: the parent is inserted before its children either way) and for
/// `true` on an unlinked one whose table elements right below it are all linked to it (XpLinkTableForceInsertUnlinked.al:551,
/// :568, :586, :603 against the control :699, :716). Other imports are refused: `false` on a linked one stores the children before
/// their parent (:667 ForceFalse_Import) and a failed insert of an unlinked one without linked children fails only at the next
/// read of the table (:621 ForceOnUnlinkedNestedChild_Import, :651 ForceOnUnlinkedParent_NoChildTable_Import); neither is
/// modelled. A FixedText port is not pinned.
fn force_insert(layout: &Layout, i: usize, reasons: &mut Vec<String>, import_reasons: &mut Vec<String>) {
    let name = &layout.nodes[i].name;
    let Some(v) = layout.property(i, "LinkTableForceInsert") else {
        return;
    };
    let linked = layout.property(i, "LinkTable").is_some();
    let value = bool_prop(&v);
    let refuse = |why: &str| format!("table element {name}: LinkTableForceInsert {why} (not pinned)");
    match (value, layout.format) {
        (None, _) | (_, al_ir::XmlFormat::FixedText) => {
            reasons.push(refuse("with this value or in a FixedText port"));
        }
        (Some(_), al_ir::XmlFormat::VariableText) => {
            if linked {
                reasons.push(refuse("on a linked table element of a VariableText port"));
            } else {
                import_reasons.push(refuse("in the import of a VariableText port"));
            }
        }
        (Some(true), al_ir::XmlFormat::Xml) if linked => {}
        (Some(true), al_ir::XmlFormat::Xml) if children_all_linked(layout, i) => {}
        (Some(_), al_ir::XmlFormat::Xml) => {
            import_reasons.push(refuse(
                "false, or on an unlinked table element without table elements linked to it below, in an import",
            ));
        }
    }
}

/// Whether table node `i` has table elements right below it and every one names it in LinkTable.
fn children_all_linked(layout: &Layout, i: usize) -> bool {
    let mut kids = (0..layout.nodes.len())
        .filter(|&k| layout.nodes[k].parent == Some(i) && layout.nodes[k].kind == al_ir::XmlNodeKind::Table)
        .peekable();
    kids.peek().is_some()
        && kids.all(|k| {
            layout
                .property(k, "LinkTable")
                .is_some_and(|t| crate::scan::unquote_xmlport(&t).eq_ignore_ascii_case(&layout.nodes[i].name))
        })
}
