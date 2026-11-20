NewLineFoundRowListRefresh => {
    id: "new-line-found-row-list-refresh",
    assumed: "On a List page, after CurrPage.SaveRecord() or CurrPage.Update(true) in the OnValidate of a control bound to a page variable moved the page from its untouched new-row line onto the stored row page code found (filter group 2 and Find('-')) and modified it, the refresh reads the view from its first row and the row again, as from the new-row line (R:B2/10000;R:B2/20000;R:B2/10000;R:B2/10000;C:B2/10000), not only the row as from a stored row. Only in the List cells of new-line-saverecord-found-row: a first CurrPage.SaveRecord() in the OnValidate, then SaveRecord or Update(true), with the same page code, filter, row and call gates. Measured on a local Business Central by the probe. Outside those cells a List keeps the refresh from a stored row.",
    tests: "No Microsoft test: al-runner issue 8523.",
    probe: "Corpus pull request 1470 (merged, not pinned yet), tests/al-language/pages/failed-update/StaleUpdateNewLine.al:1242 (SaveRecord), :1280 (Update(true)).",
    awaiting: Awaiting {
        issues: &[8523],
        files: &["tests/al-language/pages/failed-update/StaleUpdateNewLine.al"],
        since: "2026-11-20",
    },
}
