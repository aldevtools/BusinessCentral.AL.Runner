//! Container text elements (text elements with child elements) whose triggers run.

use super::Layout;
use al_ir as ir;

/// A text element with child elements whose nodes are only text attributes, leaf text elements, text elements of only text
/// attributes, and text elements of this same shape: no table, field element or field attribute below it. Its triggers run,
/// with `currXMLport.Skip()` in OnBeforePassVariable leaving out the element and all below it (xmlport/lt-y60-xmlport-container-skip/
/// Y60XpContainerSkip.al: `Period` and `Billing > InvDoc` below the root, `Ext` two levels below a table element; import of
/// `Billing > InvDoc` below the root; the same test as al-vm xmlports `container_tree`). The root's own triggers are not pinned.
pub(super) fn container_tree_shape(layout: &Layout, i: usize) -> bool {
    layout.nodes[i].parent.is_some() && tree(layout, i)
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
