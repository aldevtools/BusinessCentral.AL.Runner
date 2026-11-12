EmptyModalWorksheetSelection => {
    id: "empty-modal-worksheet-selection",
    assumed: "An action on a statically run modal Worksheet with DelayedInsert, InsertAllowed = false, no current row and one exact group-2 text filter selects the default integer key into a fresh record.",
    tests: "External repro in al-runner issue 7771, measured on every supported BC major.",
    probe: "The same tests in the pinned corpus.",
    awaiting: Awaiting {
        issues: &[7771],
        files: &["tests/al-language/pages/currpage/SelectionFilterNoRow.al::Page_SetSelectionFilter_EmptyModalWorksheet_FiltersDefaultKey"],
        since: "2026-11-11",
    },
}
