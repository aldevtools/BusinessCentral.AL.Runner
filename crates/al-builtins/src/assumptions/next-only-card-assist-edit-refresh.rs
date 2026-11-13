NextOnlyCardAssistEditRefresh => {
    id: "next-only-card-assist-edit-refresh",
    assumed: "On an OpenEdit Card whose own triggers are exactly OnNextRecord and OnAfterGetCurrRecord, with no page extension or subscriber and one linked ListPart, AssistEdit that stores and reloads the current edited row before CurrPage.Update(false) shows that record and refreshes its linked part without raising OnAfterGetCurrRecord on the Card.",
    tests: "AL Runner issue 7881 (reporter's repro AssistEditRefreshesAfterSyncStoredTheEditedRow, measured on all three supported BC majors; the issue records the exact builds).",
    probe: "Corpus pull request 1345.",
    awaiting: Awaiting {
        issues: &[7881],
        files: &["tests/al-language/pages/currpage/TestCardAssistEditSavedRefresh.al"],
        since: "2026-11-12",
    },
}
