NewLineFoundRowListRefresh => {
    id: "new-line-found-row-list-refresh",
    assumed: "On a List page, after CurrPage.SaveRecord() or CurrPage.Update(true) in the OnValidate of a control bound to a page variable moved the page from its untouched new-row line onto the stored row page code found (filter group 2 and Find('-')) and modified it, the refresh reads the view from its first row and the row again, as from the new-row line (R:B2/10000;R:B2/20000;R:B2/10000;R:B2/10000;C:B2/10000), not only the row as from a stored row. Only in the cells of new-line-saverecord-found-row (PageType List, AutoSplitKey and DelayedInsert, the six measured triggers, opened with OpenEdit, the test's calls value reads and that SetValue, none failed). Measured on Business Central 28.5. Outside those cells a List keeps the refresh from a stored row.",
    tests: "No Microsoft test: al-runner issue 8523.",
    probe: "Corpus pull request 1470 (merged, not pinned yet), tests/al-language/pages/failed-update/StaleUpdateNewLine.al:1242 (SaveRecord), :1258 (Update(true)).",
    awaiting: Awaiting {
        issues: &[8523],
        files: &["tests/al-language/pages/failed-update/StaleUpdateNewLine.al"],
        since: "2026-11-20",
    },
}
