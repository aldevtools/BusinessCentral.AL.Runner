NewDocumentBeforeVariableValidation => {
    id: "new-document-before-variable-validation",
    assumed: "On an unfiltered Document without parts, moving focus from another control by setting a variable-backed control inserts the clean draft opened by OpenNew before validation.",
    tests: "External repro in al-runner issue 7770, measured on every supported BC major.",
    probe: "The same tests in the pinned corpus.",
    awaiting: Awaiting {
        issues: &[7770],
        files: &["tests/al-language/testpage/TestPageBlankKeyInsert.al::TestPage_OpenNew_VariableControlCanModifyDocument"],
        since: "2026-11-11",
    },
}
