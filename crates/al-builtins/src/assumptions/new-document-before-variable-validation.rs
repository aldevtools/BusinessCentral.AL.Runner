NewDocumentBeforeVariableValidation => {
    id: "new-document-before-variable-validation",
    assumed: "On an insert-allowed Document opened with OpenNew, a clean draft with DelayedInsert disabled is inserted before a variable-backed control validates.",
    tests: "External repro in al-runner issue 7770, measured on real BC 27.5, 28.5 and 29.0.",
    probe: "The same tests in the pinned corpus.",
    awaiting: Awaiting {
        issues: &[7770],
        files: &["tests/al-language/testpage/TestPageBlankKeyInsert.al::TestPage_OpenNew_VariableControlCanModifyDocument"],
        since: "2026-11-11",
    },
}
