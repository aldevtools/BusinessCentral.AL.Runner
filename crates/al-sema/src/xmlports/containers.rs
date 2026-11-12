//! Container text elements (text elements with child elements) whose triggers run.

use super::Layout;
use al_ir as ir;
use ir::XmlNodeKind as K;

/// A text element with child elements whose nodes are only text attributes, leaf text elements, text elements of only text
/// attributes, and text elements of this same shape: no table, field element or field attribute below it. Its triggers run,
/// with `currXMLport.Skip()` in OnBeforePassVariable leaving out the element and all below it (xmlports/shapes-properties/
/// Y60XpContainerSkip.al: `Period` and `Billing > InvDoc` below the root, `Ext` two levels below a table element; import of
/// `Billing > InvDoc` below the root; the same test as al-vm xmlports `container_tree`). The root's own triggers are not pinned.
/// Also a text element of only text attributes whose ancestors are all text elements, no table element
/// (`attributed_outside_table`, xmlports/shapes-properties/Y67XpTextAttrSkip.al:298-374).
pub(super) fn container_tree_shape(layout: &Layout, i: usize) -> bool {
    layout.nodes[i].parent.is_some() && (tree(layout, i) || attributed_outside_table(layout, i))
}

/// A text element with text attributes only below it, every ancestor a text element (an e-document `EndpointID` below
/// containers). Its OnBeforePassVariable runs before its attributes'; Skip there leaves out the element with its attributes
/// (Y67XpTextAttrSkip.al:298 Export_NoSkip_And_SkipInElement; the same shape as al-vm xmlports `attributed_outside_table`).
fn attributed_outside_table(layout: &Layout, i: usize) -> bool {
    let mut up = layout.nodes[i].parent;
    while let Some(p) = up {
        if layout.nodes[p].kind != K::Text {
            return false;
        }
        up = layout.nodes[p].parent;
    }
    layout.nodes[i].kind == K::Text && layout.has_children(i) && attrs_only(layout, i)
}

fn tree(layout: &Layout, i: usize) -> bool {
    use ir::XmlNodeKind as K;
    let kids: Vec<usize> = (0..layout.nodes.len())
        .filter(|&k| layout.nodes[k].parent == Some(i))
        .collect();
    layout.nodes[i].kind == K::Text
        && kids.iter().any(|&k| layout.nodes[k].kind == K::Text)
        && kids.iter().all(|&k| match layout.nodes[k].kind {
            K::TextAttr => true,
            K::Text => !layout.has_children(k) || attrs_only(layout, k) || tree(layout, k),
            _ => false,
        })
}

fn attrs_only(layout: &Layout, i: usize) -> bool {
    layout
        .nodes
        .iter()
        .filter(|n| n.parent == Some(i))
        .all(|n| n.kind == ir::XmlNodeKind::TextAttr)
}

fn kids(layout: &Layout, i: usize) -> Vec<usize> {
    (0..layout.nodes.len())
        .filter(|&k| layout.nodes[k].parent == Some(i))
        .collect()
}

/// A text element whose nodes are only table elements without LinkTable (at least one).
fn table_wrap(layout: &Layout, i: usize) -> bool {
    let k = kids(layout, i);
    layout.nodes[i].kind == K::Text
        && !k.is_empty()
        && k.iter()
            .all(|&t| layout.nodes[t].kind == K::Table && layout.property(t, "LinkTable").is_none())
}

/// A text element right below a table element whose nodes are field elements (with only attributes below them, at least one),
/// `table_wrap` text elements, text elements of only text attributes and table elements without LinkTable (the last two:
/// xmlports/shapes-properties/Y67XpWrapperInTable.al:533-604 `membership`).
fn field_wrap(layout: &Layout, i: usize) -> bool {
    let k = kids(layout, i);
    layout.nodes[i].kind == K::Text
        && layout.nodes[i].parent.is_some_and(|p| layout.nodes[p].kind == K::Table)
        && k.iter().any(|&c| layout.nodes[c].kind == K::Field)
        && k.iter().all(|&c| match layout.nodes[c].kind {
            K::Field => kids(layout, c)
                .iter()
                .all(|&a| matches!(layout.nodes[a].kind, K::TextAttr | K::FieldAttr)),
            K::Text => table_wrap(layout, c) || (layout.has_children(c) && attrs_only(layout, c)),
            K::Table => layout.property(c, "LinkTable").is_none(),
            _ => false,
        })
}

/// A text element right below a table element with text attributes, leaf text and field elements (at least one) and text
/// elements of only text attributes (at least one).
fn attributed_leaves(layout: &Layout, i: usize) -> bool {
    let k = kids(layout, i);
    let leaf = |c: usize| matches!(layout.nodes[c].kind, K::Text | K::Field) && !layout.has_children(c);
    let attributed = |c: usize| layout.nodes[c].kind == K::Text && layout.has_children(c) && attrs_only(layout, c);
    layout.nodes[i].kind == K::Text
        && layout.nodes[i].parent.is_some_and(|p| layout.nodes[p].kind == K::Table)
        && k.iter().any(|&c| leaf(c))
        && k.iter().any(|&c| attributed(c))
        && k.iter()
            .all(|&c| layout.nodes[c].kind == K::TextAttr || leaf(c) || attributed(c))
}

/// The triggers of a text element inside a table element in the shapes xmlports/shapes-properties/Y60XpTextInTable.al
/// pins: a text element of field elements and wrapped table elements (`Info`, export), a wrapped table element inside it (`Sub`),
/// a text element of attributes, leaves and attributed leaves (`Ticket`, import). The same shapes as al-vm xmlports `wraps.rs`.
pub(super) fn wrap_shape(layout: &Layout, i: usize) -> bool {
    field_wrap(layout, i)
        || attributed_leaves(layout, i)
        || (table_wrap(layout, i) && layout.nodes[i].parent.is_some_and(|p| field_wrap(layout, p)))
        || (attrs_only(layout, i) && layout.nodes[i].parent.is_some_and(|p| field_wrap(layout, p)))
}

/// A text element whose nodes are only text attributes, inside a container (`tree` or `table_container_shape`) that stands, through
/// text elements, below a table element, on an export port (the PEPPOL EndpointID and CompanyID leaves,
/// streams/Y72XpEInvoiceShapes.al; the same test as al-vm xmlports einvoice_shapes.rs `attributed_leaf`).
pub(super) fn attributed_leaf_shape(layout: &Layout, i: usize) -> bool {
    let Some(p) = layout.nodes[i].parent else {
        return false;
    };
    layout.direction == ir::XmlDirection::Export
        && layout.nodes[i].kind == K::Text
        && !kids(layout, i).is_empty()
        && attrs_only(layout, i)
        && layout.nodes[p].kind == K::Text
        && (tree(layout, p) || table_container_shape(layout, p))
        && below_table(layout, p)
}

fn below_table(layout: &Layout, i: usize) -> bool {
    let mut up = layout.nodes[i].parent;
    while let Some(p) = up {
        match layout.nodes[p].kind {
            K::Table => return true,
            K::Text => up = layout.nodes[p].parent,
            _ => return false,
        }
    }
    false
}

/// A text element right below a table element, with a trigger, on an export port, whose nodes are leaf text elements, text
/// elements of only text attributes, container trees and table elements without LinkTable, at least one table element and one other
/// node (the PEPPOL TaxTotal, Item and Price, Y72XpEInvoiceShapes.al; the same test as al-vm einvoice_shapes.rs `table_container`).
pub(super) fn table_container_shape(layout: &Layout, i: usize) -> bool {
    let k = kids(layout, i);
    let table = |c: usize| layout.nodes[c].kind == K::Table && layout.property(c, "LinkTable").is_none();
    layout.direction == ir::XmlDirection::Export
        && layout.nodes[i].kind == K::Text
        && has_trigger(layout, i)
        && layout.nodes[i].parent.is_some_and(|p| layout.nodes[p].kind == K::Table)
        && k.iter().any(|&c| table(c))
        && k.iter().any(|&c| !table(c))
        && k.iter().all(|&c| {
            table(c)
                || (layout.nodes[c].kind == K::Text
                    && (!layout.has_children(c) || attrs_only(layout, c) || tree(layout, c)))
        })
}

fn has_trigger(layout: &Layout, i: usize) -> bool {
    layout.nodes[i].section.triggers().next().is_some()
}

/// The root text element with triggers whose nodes are table elements without LinkTable (at least one) and text elements that are
/// leaves, hold only text attributes or are container trees (xmlports/shapes-properties/Y83XpMiscTests.al ContainerTriggers_Nested:
/// `Root` with a table element and `Hdr`; its OnBeforePassVariable runs before everything, its OnAfterAssignVariable after
/// everything; the same test as al-vm xmlports `container_tree::root_with_table`). Only Format = Xml without InlineSchema is pinned;
/// the runtime refuses the rest.
pub(super) fn root_table_shape(layout: &Layout, i: usize) -> bool {
    let k = kids(layout, i);
    let table = |c: usize| layout.nodes[c].kind == K::Table && layout.property(c, "LinkTable").is_none();
    layout.nodes[i].parent.is_none()
        && layout.nodes[i].kind == K::Text
        && k.iter().any(|&c| table(c))
        && k.iter().all(|&c| {
            table(c)
                || (layout.nodes[c].kind == K::Text
                    && (!layout.has_children(c) || attrs_only(layout, c) || tree(layout, c)))
        })
}

/// A text element with one field attribute of its table element and OnBeforePassVariable, right below a temporary table element
/// with MinOccurs = Zero right below the root text element, on an Xml export port (corpus PR 1366,
/// xmlports/shapes-properties/XpTextElemFieldAttrExport.al:46-68; the same test as al-vm xmlports text_attr_export.rs
/// `field_attr_text`, which also checks the attribute's type).
pub(super) fn field_attr_text_shape(layout: &Layout, i: usize) -> bool {
    let n = &layout.nodes[i];
    let Some(t) = n.parent else {
        return false;
    };
    let below_root = layout.nodes[t]
        .parent
        .is_some_and(|r| layout.nodes[r].parent.is_none() && layout.nodes[r].kind == K::Text);
    let yes = |node: usize, name: &str| {
        layout
            .property(node, name)
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
    };
    let once = |node: usize, name: &str, value: &str| {
        layout
            .property(node, name)
            .is_some_and(|v| v.trim().eq_ignore_ascii_case(value))
    };
    let [attr] = kids(layout, i)[..] else {
        return false;
    };
    let a = &layout.nodes[attr];
    let of_table = a
        .args
        .get(1)
        .and_then(|s| super::split_source_at_dot(s))
        .is_some_and(|(table, _)| crate::scan::unquote_xmlport(table).eq_ignore_ascii_case(&layout.nodes[t].name));
    let mut triggers = n.section.triggers();
    let only_pass = triggers
        .next()
        .is_some_and(|r| r.name().eq_ignore_ascii_case("OnBeforePassVariable"))
        && triggers.next().is_none();
    layout.direction == ir::XmlDirection::Export
        && layout.format == ir::XmlFormat::Xml
        && n.kind == K::Text
        && layout.nodes[t].kind == K::Table
        && yes(t, "UseTemporary")
        && once(t, "MinOccurs", "Zero")
        && below_root
        && once(i, "MaxOccurs", "Once")
        && layout.property(i, "NamespacePrefix").is_none()
        && only_pass
        && a.kind == K::FieldAttr
        && of_table
        && layout.property(attr, "Occurrence").is_none()
        && a.section.triggers().next().is_none()
}
