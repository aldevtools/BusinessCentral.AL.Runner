use super::*;

pub(super) fn note_explicit_parent_link(run: &Run, r: ItemRef) {
    if let ItemRef::Added(extension, item) = r
        && run
            .ext_ir(extension)
            .is_some_and(|ext| ext.added[item].explicit_parent_link)
    {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ReportExtensionExplicitParentLink);
    }
}
