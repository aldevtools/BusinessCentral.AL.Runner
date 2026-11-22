use super::*;

/// The row a data item links to: the base item `DataItemLinkReference` names (`link_item`) among the enclosing ones, else
/// the parent's. The explicit links of added items are pinned at TestReportExtensionParentLink.al:46-61 (al-sema
/// `reports.rs` refuses the other anchors).
pub(super) fn linked_parent(
    link_item: Option<usize>,
    parent: Option<Rec>,
    enclosing: &[(ItemRef, Rec)],
) -> Option<Rec> {
    match link_item {
        Some(index) => enclosing
            .iter()
            .find(|(item, _)| matches!(item, ItemRef::Base(base) if *base == index))
            .map(|(_, record)| record.clone())
            .or(parent),
        None => parent,
    }
}
