//! Container text elements (text elements with child elements) whose triggers run.

use super::Layout;
use al_ir as ir;
use ir::XmlNodeKind as K;

/// A text element with child elements whose nodes are only text attributes, leaf text elements, text elements of only text
/// attributes, and text elements of this same shape: no table, field element or field attribute below it. Its triggers run,
/// with `currXMLport.Skip()` in OnBeforePassVariable leaving out the element and all below it (xmlport/lt-y60-xmlport-container-skip/
/// Y60XpContainerSkip.al: `Period` and `Billing > InvDoc` below the root, `Ext` two levels below a table element; import of
/// `Billing > InvDoc` below the root; the same test as al-vm xmlports `container_tree`). The root's own triggers are not pinned.
/// Also a text element of only text attributes whose ancestors are all text elements, no table element
/// (`attributed_outside_table`, xmlport/lt-y67-xmlport-textattr-skip/Y67XpTextAttrSkip.al:298-374).
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
/// xmlport/lt-y67-xmlport-wrapper-in-table/Y67XpWrapperInTable.al:533-604 `membership`).
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

/// The triggers of a text element inside a table element in the shapes xmlport/lt-y60-xmlport-text-in-table/Y60XpTextInTable.al
/// pins: a text element of field elements and wrapped table elements (`Info`, export), a wrapped table element inside it (`Sub`),
/// a text element of attributes, leaves and attributed leaves (`Ticket`, import). The same shapes as al-vm xmlports `wraps.rs`.
pub(super) fn wrap_shape(layout: &Layout, i: usize) -> bool {
    field_wrap(layout, i)
        || attributed_leaves(layout, i)
        || (table_wrap(layout, i) && layout.nodes[i].parent.is_some_and(|p| field_wrap(layout, p)))
        || (attrs_only(layout, i) && layout.nodes[i].parent.is_some_and(|p| field_wrap(layout, p)))
}
