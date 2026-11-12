EmptyWorksheetUpdateSelectsRow => {
    id: "empty-worksheet-update-selects-row",
    assumed: "Update(false) after an action that fills an empty nonmodal Worksheet with inserts disabled makes its one visible row selectable.",
    tests: "Local neutral probe WorksheetRefresh.al, measured on all supported BC majors.",
    probe: "Add the measured case to the pinned corpus.",
    awaiting: Awaiting {
        issues: &[7789],
        files: &["tests/al-language/pages/currpage/SelectionFilterNoRow.al::TestPage_Update_NewWorksheetRow_IsSelectable"],
        since: "2026-11-11",
    },
}
