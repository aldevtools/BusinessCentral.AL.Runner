NewLineSaveRecordFoundRow => {
    id: "new-line-saverecord-found-row",
    assumed: "CurrPage.SaveRecord() in the OnValidate of a control bound to a page variable, on the untouched new-row line of a page the test opened with OpenEdit, after page code moved Rec onto a stored row (a filter in filter group 2 and Find('-')): the page leaves the line and modifies that row inside the OnValidate (OnModifyRecord with xRec the blank new line, then the table's OnModify; a value page code set in Rec is stored), then refreshes as after CurrPage.Update(true). Only on a top-level Worksheet or List page over a database table that declares PageType, SourceTable, AutoSplitKey and DelayedInsert (both true), ApplicationArea and UsageCategory, and exactly the triggers OnOpenPage, OnAfterGetRecord, OnAfterGetCurrRecord, OnNewRecord, OnModifyRecord and OnInsertRecord, with no part, page extension or event subscriber, when the test's calls since the open are value reads and that SetValue and none of them failed. Measured on Business Central 28.5. Every other SaveRecord in this place is refused; a failed earlier step (blank page, no reads after the save) is not run.",
    tests: "No Microsoft test: al-runner issue 8523.",
    probe: "Corpus pull request 1470 (merged, not pinned yet), tests/al-language/pages/failed-update/StaleUpdateNewLine.al:498 (Worksheet, first SaveRecord), :914 (without the first SaveRecord), :1242 (List), :1318 (Description set by code).",
    awaiting: Awaiting {
        issues: &[8523],
        files: &["tests/al-language/pages/failed-update/StaleUpdateNewLine.al"],
        since: "2026-11-20",
    },
}
