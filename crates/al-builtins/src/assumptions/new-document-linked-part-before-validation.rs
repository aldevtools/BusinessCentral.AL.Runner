NewDocumentLinkedPartBeforeValidation => {
    id: "new-document-linked-part-before-validation",
    assumed: "A clean OpenNew Document with one empty linked ListPart inserts its header before a variable control validates on a focus move.",
    tests: "Local neutral probe DocumentWithPart.al, measured on all supported BC majors.",
    probe: "Add the measured case to the pinned corpus.",
    awaiting: Awaiting {
        issues: &[7788],
        files: &["tests/al-language/testpage/TestPageBlankKeyInsert.al::TestPage_OpenNew_VariableControlCanModifyDocumentWithPart"],
        since: "2026-11-11",
    },
}
