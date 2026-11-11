EmptyModalWorksheetSelection => {
    id: "empty-modal-worksheet-selection",
    assumed: "On a filtered modal Worksheet with InsertAllowed = false and no current row, an action's SetSelectionFilter into a fresh record selects the default key without marking a row.",
    tests: "External repro in al-runner issue 7771, measured on real BC 27.5, 28.5 and 29.0.",
    probe: "The same tests in the pinned corpus.",
    awaiting: Awaiting {
        issues: &[7771],
        files: &["tests/al-language/pages/currpage/SelectionFilterNoRow.al::Page_SetSelectionFilter_EmptyModalWorksheet_FiltersDefaultKey"],
        since: "2026-11-11",
    },
}
