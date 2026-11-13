FilteredModalWorksheetBlankCodeSelection => {
    id: "filtered-modal-worksheet-blank-code-selection",
    assumed: "A static modal Worksheet with DelayedInsert, SaveValues, InsertAllowed = false and a source view selects its blank Code key when its action calls SetSelectionFilter with two non-key equality filters in group 2.",
    tests: "al-runner issue 7771 and corpus PR #1354, pages/run-modal/TestModalWorksheetBlankCode.al (BC 27.5, 28.5 and 29.0).",
    probe: "Corpus PR #1354, pages/run-modal/TestModalWorksheetBlankCode.al.",
    awaiting: Awaiting {
        issues: &[7771],
        files: &["tests/al-language/pages/run-modal/TestModalWorksheetBlankCode.al"],
        since: "2026-11-13",
    },
}
