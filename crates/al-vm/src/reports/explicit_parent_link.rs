use super::*;

/// The measured explicit parent links are in corpus PR #1358, reports/extensions/TestReportExtensionParentLink.al:47-61.
pub(super) fn linked_parent(
    run: &Run,
    r: ItemRef,
    link_item: Option<usize>,
    parent: Option<Rec>,
    enclosing: &[(ItemRef, Rec)],
) -> Option<Rec> {
    if let ItemRef::Added(extension, item) = r
        && run
            .ext_ir(extension)
            .is_some_and(|ext| ext.added[item].explicit_parent_link)
    {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ReportExtensionExplicitParentLink);
    }
    match link_item {
        Some(index) => enclosing
            .iter()
            .find(|(item, _)| matches!(item, ItemRef::Base(base) if *base == index))
            .map(|(_, record)| record.clone())
            .or(parent),
        None => parent,
    }
}
