//! Export shapes of e-invoice ports (PEPPOL, Fin. Results) beyond the container trees of `container_tree.rs`
//! (xmlport/lt-y72-xmlport-probes/Y72XpEInvoiceShapes.al; the same shapes as al-sema xmlports/containers.rs `attributed_leaf_shape`
//! and `table_container_shape`).

use super::*;

/// A text element whose nodes are only text attributes, with its own value, inside a container (`container_tree` or
/// `table_container`) that stands, through containers, below a table element: written as a text element right below a table element
/// is (its OnBeforePassVariable first, Skip there leaves it out with its attributes, whose triggers do not run; an attribute whose
/// trigger skips is left out alone). Pinned by Y72XpEInvoiceShapes.al DeepAttrLeaf_AllFilled, DeepAttrLeaf_SkipVariants (EndpointID,
/// CompanyID with its own attribute trigger, two and three containers below the table element), DeepAttrLeaf_RealTableRoot_*.
pub(super) fn attributed_leaf(schema: &Schema, c: usize) -> bool {
    let Some(p) = schema.node(c).parent else {
        return false;
    };
    schema.node(c).kind == ir::XmlNodeKind::Text
        && attrs_only(schema, c)
        && schema.node(p).kind == ir::XmlNodeKind::Text
        && (container_tree(schema, p) || table_container(schema, p))
        && below_table(schema, p)
}

/// Whether text element `c` stands below a table element through text elements only.
fn below_table(schema: &Schema, c: usize) -> bool {
    let mut up = schema.node(c).parent;
    while let Some(p) = up {
        match schema.node(p).kind {
            ir::XmlNodeKind::Table => return true,
            ir::XmlNodeKind::Text => up = schema.node(p).parent,
            _ => return false,
        }
    }
    false
}

/// A text element right below a table element, with a trigger, whose nodes are leaf text elements, text elements of only text
/// attributes, container trees and table elements without LinkTable (at least one table element and one other node): TaxTotal,
/// Item and Price of the PEPPOL ports. Its OnBeforePassVariable runs first and Skip there leaves it out with all below it (the inner
/// table elements' triggers do not run); then its nodes in schema order, an inner table element with its OnPreXmlItem once and its
/// rows (Break ends it, Skip leaves out the row); with no inner row it is still written with its leaves (Y72XpEInvoiceShapes.al
/// TableContainer_TaxTotal, TableContainer_ItemAndPrice, TableContainer_SkipContainer, TableContainer_BreakAndSkipInInnerTable).
pub(super) fn table_container(schema: &Schema, c: usize) -> bool {
    let n = schema.node(c);
    let kids = schema.children(c);
    let table = |k: usize| schema.node(k).kind == ir::XmlNodeKind::Table && schema.node(k).link_node.is_none();
    n.kind == ir::XmlNodeKind::Text
        && !n.triggers.is_empty()
        && n.parent.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table)
        && kids.iter().any(|&k| table(k))
        && kids.iter().any(|&k| !table(k))
        && kids.iter().all(|&k| {
            table(k)
                || (schema.node(k).kind == ir::XmlNodeKind::Text
                    && (schema.children(k).is_empty() || attrs_only(schema, k) || container_tree(schema, k)))
        })
}

/// A `container_tree` that holds, at any depth, an `attributed_leaf` with a trigger (Supplier > Party > EndpointID): written as a
/// triggered container is, so the leaf's trigger runs (Y72XpEInvoiceShapes.al DeepAttrLeaf_AllFilled: AccountingSupplierParty).
pub(super) fn tree_with_triggered_leaf(schema: &Schema, c: usize) -> bool {
    fn holds(schema: &Schema, c: usize) -> bool {
        schema.children(c).into_iter().any(|k| {
            schema.node(k).kind == ir::XmlNodeKind::Text
                && ((attrs_only(schema, k) && !schema.node(k).triggers.is_empty()) || holds(schema, k))
        })
    }
    container_tree(schema, c) && below_table(schema, c) && holds(schema, c)
}
